use crate::{ALL, ExperimentalOption, Stability};
use std::{borrow::Cow, env, ops::Range, sync::atomic::Ordering};
use thiserror::Error;

pub const ENV: &str = "NU_EXPERIMENTAL_OPTIONS";

#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum ParseWarning {
    #[error("Unknown experimental option `{0}`")]
    Unknown(String),
    #[error("Invalid assignment for `{identifier}`, expected `true` or `false`, got `{1}`", identifier = .0.identifier())]
    InvalidAssignment(&'static ExperimentalOption, String),
    #[error("The experimental option `{identifier}` is deprecated and will be removed in a future release", identifier = .0.identifier())]
    Deprecated(&'static ExperimentalOption),
}

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
