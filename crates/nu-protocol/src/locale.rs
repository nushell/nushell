use std::{fmt, str::FromStr};
use thiserror::Error;

// TODO: this should be reworked. Currently, this contains both a `chrono::Locale` and a
// `num_format::Locale` so that the same `Locale` type can be used to format numbers and dates.
// In the future, we should possibly look into using `ICU4X` via the `icu` crate(s) to unify
// the locale types as well as for better Intl.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locale {
    number: num_format::Locale,
    date: chrono::Locale,
}

#[allow(dead_code)]
fn env(key: &'static str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

impl Locale {
    pub const OVERRIDE_ENV_VAR: &str = "NU_TEST_LOCALE_OVERRIDE";

    #[inline]
    pub fn name(&self) -> &'static str {
        self.number.name()
    }

    fn locale_override() -> Option<Self> {
        #[cfg(debug_assertions)]
        {
            if let Some(s) = env(Locale::OVERRIDE_ENV_VAR) {
                return s.parse().ok();
            }
        }

        #[cfg(test)]
        {
            // For tests, we use the same locale on all systems.
            // To override this, set `OVERRIDE_ENV_VAR`.
            Some(Locale::default())
        }

        #[cfg(not(test))]
        {
            None
        }
    }

    pub fn system_number() -> Option<Self> {
        Self::locale_override().or_else(system::number)
    }

    pub fn system_date() -> Option<Self> {
        Self::locale_override().or_else(system::date)
    }

    // TODO: this should not be public. Formatting functions should be moved inside this crate.
    pub fn number(&self) -> num_format::Locale {
        self.number
    }

    // TODO: this should not be public. Formatting functions should be moved inside this crate.
    pub fn date(&self) -> chrono::Locale {
        self.date
    }
}

#[cfg(all(unix, not(any(target_vendor = "apple", target_os = "android"))))]
mod system {
    use super::{env, Locale};

    fn unix_locale(lc_var: &'static str) -> Option<Locale> {
        // The code below is modified from `sys_locale::get_locale`. Instead of `LC_MESSAGES`,
        // we use the passed in `lc_var`. This is to allow using `LC_TIME` for the date locale and
        // `LC_NUMERIC` for the number locale.
        if let Some(langs) = env("LANGUAGE") {
            for lang in langs.split(':') {
                if let Ok(locale) = lang.parse() {
                    return Some(locale);
                }
            }
        }

        ["LC_ALL", lc_var, "LANG"]
            .iter()
            .find_map(|var| env(var).and_then(|s| s.parse().ok()))
    }

    pub fn number() -> Option<Locale> {
        unix_locale("LC_NUMERIC")
    }

    pub fn date() -> Option<Locale> {
        unix_locale("LC_TIME")
    }
}

#[cfg(not(all(unix, not(any(target_vendor = "apple", target_os = "android")))))]
mod system {
    use super::Locale;

    fn sys_locale() -> Option<Locale> {
        sys_locale::get_locale()?.parse().ok()
    }

    pub fn number() -> Option<Locale> {
        sys_locale()
    }

    pub fn date() -> Option<Locale> {
        sys_locale()
    }
}

impl Default for Locale {
    #[inline]
    fn default() -> Self {
        Self {
            number: num_format::Locale::en_US_POSIX,
            date: chrono::Locale::POSIX,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The error returned when failing to parse a [`Locale`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Error)]
pub struct ParseLocaleError(());

impl fmt::Display for ParseLocaleError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "unknown locale")
    }
}

impl FromStr for Locale {
    type Err = ParseLocaleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Remove anything after the first "." like in "en_US.UTF-8".
        let s = s.split_once(".").map(|(s, _)| s).unwrap_or(s);

        let number = s
            .parse::<num_format::Locale>()
            .ok()
            .or_else(|| {
                // `num_format::Locale` is missing some language and region combinations,
                // so we fallback to only the language if possible (e.g., "en-US" => "en").
                // Note that `num_format::Locale` allows both "-" and "_" when parsing identifiers.
                s.split_once(['_', '-'])?.0.parse().ok()
            })
            .ok_or(ParseLocaleError(()))?;

        let date = if number == num_format::Locale::en_US_POSIX {
            chrono::Locale::POSIX
        } else {
            // `chrono::Locale` only allows "_" in identifiers. If parsing fails, we try again with
            // "-" replaced by "_". If that fails too, we fallback to the default.
            s.parse::<chrono::Locale>()
                .ok()
                .or_else(|| s.replace("-", "_").parse().ok())
                .unwrap_or(Self::default().date())
        };

        Ok(Self { number, date })
    }
}
