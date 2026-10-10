use crate::{ALL, ExperimentalOption, Status};
use itertools::Itertools;
use std::{
    borrow::Cow,
    env,
    ops::Range,
    sync::{OnceLock, atomic::Ordering},
};
use thiserror::Error;

/// Environment variable used to load experimental options from.
///
/// May be used like this: `NU_EXPERIMENTAL_OPTIONS=example nu`.
pub const ENV: &str = "NU_EXPERIMENTAL_OPTIONS";

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
/// When the key `"all"` is encountered, every experimental option that isn't deprecated is set,
/// as [`set_all`](super::set_all) does.
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
    let (assignments, warnings) = resolve(iter);
    for (option, val) in assignments {
        option.value.store(val, Ordering::Relaxed);
    }
    warnings
}

/// An option and the value assigned to it.
type Assignment = (&'static ExperimentalOption, bool);

/// The assignments that `iter` makes, in order (a later one overrides an earlier one for the same
/// option), without applying them. `all` expands to every option that isn't deprecated, as
/// [`set_all`](super::set_all) does.
fn resolve<'i, Ctx: Clone>(
    iter: impl Iterator<Item = (Cow<'i, str>, Option<Cow<'i, str>>, Ctx)>,
) -> (Vec<Assignment>, Vec<(ParseWarning, Ctx)>) {
    let mut assignments = Vec::new();
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
            assignments.extend(
                ALL.iter()
                    .filter(|option| !option.status().is_deprecated())
                    .map(|option| (*option, val)),
            );
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

        assignments.push((*option, val));
    }

    (assignments, warnings)
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
    parse_iter(env_entries(&env))
}

/// Split the value of [`ENV`] into `(key, value, range)` entries for [`parse_iter`]/[`resolve`].
fn env_entries(
    env: &str,
) -> impl Iterator<Item = (Cow<'_, str>, Option<Cow<'_, str>>, Range<usize>)> {
    let mut entries = Vec::new();
    let mut start = 0;
    for (idx, c) in env.char_indices() {
        if c == ',' {
            entries.push((&env[start..idx], start..idx));
            start = idx + 1;
        }
    }
    entries.push((&env[start..], start..env.len()));

    entries.into_iter().map(|(entry, span)| {
        entry
            .split_once("=")
            .map(|(key, val)| (key.into(), Some(val.into()), span.clone()))
            .unwrap_or((entry.into(), None, span))
    })
}

/// The value [`ENV`] assigns to `option`, if any.
///
/// [`ExperimentalOption::get`] falls back to this for options that were never set, so the
/// environment variable applies even where [`parse_env`] isn't called: in embedders and in test
/// binaries (whose harness resets every option before each test group). The variable is read
/// once per process and resolved as [`parse_env`] resolves it (unknown options and invalid
/// values are skipped, deprecated options still apply), without reporting warnings.
pub(crate) fn env_value(option: &ExperimentalOption) -> Option<bool> {
    static ENV_ASSIGNMENTS: OnceLock<Vec<Assignment>> = OnceLock::new();
    ENV_ASSIGNMENTS
        .get_or_init(|| match env::var(ENV) {
            Ok(env) => resolve(env_entries(&env)).0,
            Err(_) => Vec::new(),
        })
        .iter()
        .rev()
        .find(|(assigned, _)| *assigned == option)
        .map(|(_, val)| *val)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(env: &str) -> Vec<(&'static str, bool)> {
        resolve(env_entries(env))
            .0
            .into_iter()
            .map(|(option, val)| (option.identifier(), val))
            .collect()
    }

    #[test]
    fn resolve_keeps_assignments_in_order() {
        assert_eq!(
            resolved("pipefail=false,winnow-parser,pipefail"),
            [
                ("pipefail", false),
                ("winnow-parser", true),
                ("pipefail", true)
            ]
        );
    }

    #[test]
    fn resolve_expands_all_to_options_that_are_not_deprecated() {
        let expected: Vec<_> = crate::ALL
            .iter()
            .filter(|option| !option.status().is_deprecated())
            .map(|option| (option.identifier(), false))
            .collect();
        assert_eq!(resolved("all=false"), expected);
    }

    #[test]
    fn resolve_skips_invalid_entries() {
        let (assignments, warnings) = resolve(env_entries("nope,winnow-parser=maybe,all=perhaps"));
        assert!(assignments.is_empty());
        assert_eq!(warnings.len(), 3);
    }

    #[test]
    fn resolve_applies_deprecated_options_with_a_warning() {
        // `example` is deprecated: named on its own it still applies, as in `parse_env`.
        assert_eq!(resolved("example"), [("example", true)]);
        assert_eq!(resolve(env_entries("example")).1.len(), 1);
    }
}
