use crate::{ALL, ExperimentalOption, Status};
use itertools::Itertools;
use std::{borrow::Cow, env, ops::Range, sync::OnceLock, sync::atomic::Ordering};
use thiserror::Error;

/// Environment variable used to load experimental options from.
///
/// May be used like this: `NU_EXPERIMENTAL_OPTIONS=example nu`.
pub const ENV: &str = "NU_EXPERIMENTAL_OPTIONS";

static STARTUP_BASELINE: OnceLock<Vec<bool>> = OnceLock::new();

/// Warnings that can happen while parsing experimental options.
#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum ParseWarning {
    /// The given identifier doesn't match any known experimental option.
    #[error("Unknown experimental option `{0}`")]
    Unknown(String),

    /// The assignment wasn't valid. Only `true` or `false` is accepted.
    #[error("Invalid assignment for `{identifier}`, expected `true` or `false`, got `{1}`", identifier = .0.identifier())]
    InvalidAssignment(&'static ExperimentalOption, String),

    /// The assignment for "all" wasn't valid. Only `true` or `false` is accepted.
    #[error("Invalid assignment for `all`, expected `true` or `false`, got `{0}`")]
    InvalidAssignmentAll(String),

    /// This experimental option is deprecated as this is now the default behavior.
    #[error("The experimental option `{identifier}` is deprecated as this is now the default behavior.", identifier = .0.identifier())]
    DeprecatedDefault(&'static ExperimentalOption),

    /// This experimental option is deprecated and will be removed in the future.
    #[error("The experimental option `{identifier}` is deprecated and will be removed in a future release", identifier = .0.identifier())]
    DeprecatedDiscard(&'static ExperimentalOption),
}

/// Parse and activate experimental options.
///
/// This is the recommended way to activate options, as it handles [`ParseWarning`]s properly
/// and is easy to hook into.
///
/// When the key `"all"` is encountered, [`set_all`](super::set_all) is used to set all
/// experimental options that aren't deprecated.
/// This allows opting (or opting out of) all experimental options that are currently available for
/// testing.
///
/// The `iter` argument should yield:
/// - the identifier of the option
/// - an optional assignment value (`true`/`false`)
/// - a context value, which is returned with any warning
///
/// This way you don't need to manually track which input caused which warning.
pub fn parse_iter<'i, Ctx: Clone>(
    iter: impl Iterator<Item = (Cow<'i, str>, Option<Cow<'i, str>>, Ctx)>,
) -> Vec<(ParseWarning, Ctx)> {
    let mut warnings = Vec::new();
    for (key, val, ctx) in iter {
        if key == "all" {
            let val = match parse_val(val.as_deref()) {
                Ok(val) => val,
                Err(s) => {
                    warnings.push((ParseWarning::InvalidAssignmentAll(s.to_owned()), ctx));
                    continue;
                }
            };
            super::set_all(val);
            continue;
        }

        let Some(option) = ALL.iter().find(|option| option.identifier() == key.trim()) else {
            warnings.push((ParseWarning::Unknown(key.to_string()), ctx));
            continue;
        };

        match option.status() {
            Status::DeprecatedDiscard => {
                warnings.push((ParseWarning::DeprecatedDiscard(option), ctx.clone()));
            }
            Status::DeprecatedDefault => {
                warnings.push((ParseWarning::DeprecatedDefault(option), ctx.clone()));
            }
            _ => {}
        }

        let val = match parse_val(val.as_deref()) {
            Ok(val) => val,
            Err(s) => {
                warnings.push((ParseWarning::InvalidAssignment(option, s.to_owned()), ctx));
                continue;
            }
        };

        option.value.store(val, Ordering::Relaxed);
    }

    warnings
}

fn parse_val(val: Option<&str>) -> Result<bool, &str> {
    match val.map(str::trim) {
        None => Ok(true),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(s) => Err(s),
    }
}

/// Parse experimental options from the [`ENV`] environment variable.
///
/// Uses [`parse_iter`] internally. Each warning includes a `Range<usize>` pointing to the
/// part of the environment variable that triggered it.
pub fn parse_env() -> Vec<(ParseWarning, Range<usize>)> {
    let Ok(env) = env::var(ENV) else {
        return vec![];
    };

    parse_value(&env)
}

/// Parse experimental options from a single string value.
///
/// Uses [`parse_iter`] internally. Each warning includes a `Range<usize>` pointing to the
/// substring that triggered it.
pub fn parse_value(value: &str) -> Vec<(ParseWarning, Range<usize>)> {
    let entries = split_entries(value);

    parse_iter(entries.into_iter().map(|(entry, span)| {
        entry
            .split_once("=")
            .map(|(key, val)| (key.into(), Some(val.into()), span.clone()))
            .unwrap_or((entry.into(), None, span))
    }))
}

/// Apply a runtime value as a full replacement.
///
/// This resets all options to default state before parsing the new value, so unspecified options go back to defaults.
pub fn apply_runtime_value(value: &str) -> Vec<(ParseWarning, Range<usize>)> {
    crate::reset_all();
    parse_value(value)
}

/// Save the current option states as startup baseline.
///
/// This should be called once after startup options (env + cli) are fully applied.
pub fn snapshot_startup_baseline() {
    let _ = STARTUP_BASELINE.set(crate::snapshot_values());
}

/// Restore the startup baseline if available, else restore defaults.
pub fn restore_startup_baseline() {
    if let Some(values) = STARTUP_BASELINE.get() {
        crate::restore_values(values);
    } else {
        crate::reset_all();
    }
}

fn split_entries(value: &str) -> Vec<(&str, Range<usize>)> {
    let mut entries = Vec::new();

    let mut start = 0;
    for (idx, c) in value.char_indices() {
        if c == ',' {
            entries.push((&value[start..idx], start..idx));
            start = idx + 1;
        }
    }
    entries.push((&value[start..], start..value.len()));

    entries
}

impl ParseWarning {
    /// A code to represent the variant.
    ///
    /// This may be used with crates like [`miette`](https://docs.rs/miette) to provide error codes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unknown(_) => "nu::experimental_option::unknown",
            Self::InvalidAssignment(_, _) => "nu::experimental_option::invalid_assignment",
            Self::InvalidAssignmentAll(_) => "nu::experimental_option::invalid_assignment_all",
            Self::DeprecatedDefault(_) => "nu::experimental_option::deprecated_default",
            Self::DeprecatedDiscard(_) => "nu::experimental_option::deprecated_discard",
        }
    }

    /// Provide some help depending on the variant.
    ///
    /// This may be used with crates like [`miette`](https://docs.rs/miette) to provide a help
    /// message.
    pub fn help(&self) -> Option<String> {
        match self {
            Self::Unknown(_) => Some(format!(
                "Known experimental options are: {}",
                ALL.iter().map(|option| option.identifier()).join(", ")
            )),
            Self::InvalidAssignment(_, _) => None,
            Self::InvalidAssignmentAll(_) => None,
            Self::DeprecatedDiscard(_) => None,
            Self::DeprecatedDefault(_) => {
                Some(String::from("You can safely remove this option now."))
            }
        }
    }
}
