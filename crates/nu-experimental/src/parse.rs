use crate::{ALL, ExperimentalOption, Stability};
use std::{borrow::Cow, env, ops::Range, sync::atomic::Ordering};
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
    
    /// This experimental option is deprecated and will be removed in the future.
    #[error("The experimental option `{identifier}` is deprecated and will be removed in a future release", identifier = .0.identifier())]
    Deprecated(&'static ExperimentalOption),
}

/// Parse and activate experimental options.
///
/// This is the recommended way to activate options, as it handles [`ParseWarning`]s properly
/// and is easy to hook into.
///
/// The `iter` argument should yield:
/// - the identifier of the option
/// - an optional assignment value (`true`/`false`)
/// - a context value, which is returned with any warning
///
/// This way you don't need to manually track which input caused which warning.
pub fn parse_iter<'i, Ctx>(
    iter: impl Iterator<Item = (Cow<'i, str>, Option<Cow<'i, str>>, Ctx)>,
) -> Vec<(ParseWarning, Ctx)> {
    let mut warnings = Vec::new();
    'entries: for (key, val, ctx) in iter {
        for option in ALL {
            if option.identifier() == key.trim() {
                if option.stability() == Stability::Deprecated {
                    warnings.push((ParseWarning::Deprecated(option), ctx));
                    continue 'entries;
                }

                let val = match val.as_ref().map(|s| s.trim()) {
                    None => true,
                    Some("true") => true,
                    Some("false") => false,
                    Some(s) => {
                        warnings.push((ParseWarning::InvalidAssignment(option, s.to_owned()), ctx));
                        continue 'entries;
                    }
                };

                option.value.store(val, Ordering::Relaxed);
                continue 'entries;
            }
        }

        warnings.push((ParseWarning::Unknown(key.to_string()), ctx));
    }

    warnings
}

/// Parse experimental options from the [`ENV`] environment variable.
///
/// Uses [`parse_iter`] internally. Each warning includes a `Range<usize>` pointing to the
/// part of the environment variable that triggered it.
pub fn parse_env() -> Vec<(ParseWarning, Range<usize>)> {
    let Ok(env) = env::var(ENV) else {
        return vec![];
    };

    let mut entries = Vec::new();
    let mut start = 0;
    for (idx, c) in env.char_indices() {
        if c == ',' {
            entries.push((&env[start..idx], start..idx));
            start = idx + 1;
        }
    }
    entries.push((&env[start..], start..env.len()));

    parse_iter(entries.into_iter().map(|(entry, span)| {
        entry
            .split_once("=")
            .map(|(key, val)| (key.into(), Some(val.into()), span.clone()))
            .unwrap_or((entry.into(), None, span))
    }))
}
