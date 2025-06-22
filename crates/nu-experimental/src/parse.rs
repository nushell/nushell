use crate::{ALL, ExperimentalOption, Stability};
use std::{borrow::Cow, env, sync::atomic::Ordering};
use thiserror::Error;

pub const ENV: &str = "NU_EXPERIMENTS";

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

pub fn parse_env() -> Vec<(ParseWarning, ())> {
    let Ok(env) = env::var(ENV) else {
        return vec![];
    };

    parse_iter(env.split(",").map(|entry| {
        entry
            .split_once("=")
            .map(|(key, val)| (key.into(), Some(val.into()), ()))
            .unwrap_or((entry.into(), None, ()))
    }))
}
