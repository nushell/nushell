use std::{borrow::Cow, env};

use crate::{ALL, ExperimentalOption, Stability};

pub const ENV: &str = "NU_EXPERIMENTS";

pub enum ParseWarning {
    Unknown(String),
    InvalidAssignment(&'static ExperimentalOption, String),
    Deprecated(&'static ExperimentalOption),
}

pub fn parse_iter<'i>(
    iter: impl Iterator<Item = (Cow<'i, str>, Option<Cow<'i, str>>)>,
) -> Vec<ParseWarning> {
    let mut warnings = Vec::new();
    'entries: for (key, val) in iter {
        for option in ALL {
            if option.identifier() == key.trim() {
                if option.stability() == Stability::Deprecated {
                    warnings.push(ParseWarning::Deprecated(option));
                    continue 'entries;
                }

                let val = match val.as_ref().map(|s| s.trim()) {
                    None => true,
                    Some("true") => true,
                    Some("false") => false,
                    Some(s) => {
                        warnings.push(ParseWarning::InvalidAssignment(option, s.to_owned()));
                        continue 'entries;
                    }
                };

                option.set(val);
                continue 'entries;
            }
        }

        warnings.push(ParseWarning::Unknown(key.to_string()));
    }

    warnings
}

pub fn parse_env() -> Vec<ParseWarning> {
    let Ok(env) = env::var(ENV) else {
        return vec![];
    };

    parse_iter(env.split(",").map(|entry| {
        entry
            .split_once("=")
            .map(|(key, val)| (key.into(), Some(val.into())))
            .unwrap_or((entry.into(), None))
    }))
}
