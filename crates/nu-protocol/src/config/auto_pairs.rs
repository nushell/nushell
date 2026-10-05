use super::prelude::*;
use crate::{self as nu_protocol, ConfigWarning};

/// Configuration for automatic pair insertion in the line editor (`$env.config.auto_pairs`).
#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPairsConfig {
    pub enable: bool,
    pub pairs: Vec<AutoPair>,
    pub also: AutoPairsAlso,
}

impl Default for AutoPairsConfig {
    fn default() -> Self {
        Self {
            enable: false,
            pairs: [
                ('(', ')'),
                ('[', ']'),
                ('{', '}'),
                ('"', '"'),
                ('\'', '\''),
                ('`', '`'),
            ]
            .map(|(open, close)| AutoPair { open, close })
            .into(),
            also: AutoPairsAlso::default(),
        }
    }
}

impl UpdateFromValue for AutoPairsConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "enable" => self.enable.update(val, path, errors),
                "pairs" => self.pairs.update(val, path, errors),
                "also" => self.also.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }

        let missing = record
            .get("also")
            .and_then(|also| also.as_record().ok())
            .into_iter()
            .flat_map(|also| also.values())
            .filter_map(|list| list.as_list().ok())
            .flatten()
            .filter(|val| {
                val.as_str()
                    .ok()
                    .and_then(|str| str.parse::<AutoPair>().ok())
                    .is_some_and(|pair| !self.pairs.contains(&pair))
            });
        for val in missing {
            errors.warn(ConfigWarning::IncompatibleOptions {
                label: "this pair is not in auto_pairs.pairs, so it has no effect",
                span: val.span(),
                help: "add it to $env.config.auto_pairs.pairs, or remove it from $env.config.auto_pairs.also",
            });
        }
    }
}

/// Pairs that are also inserted where they are not by default (`$env.config.auto_pairs.also`).
#[derive(Clone, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPairsAlso {
    pub in_string: Vec<AutoPair>,
    pub in_comment: Vec<AutoPair>,
    pub after_word: Vec<AutoPair>,
    pub before_text: Vec<AutoPair>,
}

impl UpdateFromValue for AutoPairsAlso {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "in_string" => self.in_string.update(val, path, errors),
                "in_comment" => self.in_comment.update(val, path, errors),
                "after_word" => self.after_word.update(val, path, errors),
                "before_text" => self.before_text.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

/// An opening and a closing character, written as a two-character string such as `"()"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoPair {
    pub open: char,
    pub close: char,
}

impl IntoValue for AutoPair {
    fn into_value(self, span: Span) -> Value {
        Value::string(String::from_iter([self.open, self.close]), span)
    }
}

impl FromStr for AutoPair {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        match (chars.next(), chars.next(), chars.next()) {
            (Some(open), Some(close), None) => Ok(Self { open, close }),
            _ => Err("a string of two characters like '()'"),
        }
    }
}

impl UpdateFromValue for Vec<AutoPair> {
    /// Keeps the old list if any element is invalid, so that a typo does not drop the other pairs.
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Ok(list) = value.as_list() else {
            errors.type_mismatch(path, Type::list(Type::String), value);
            return;
        };

        let pairs: Vec<_> = list
            .iter()
            .filter_map(|val| match val.as_str().map(str::parse) {
                Ok(Ok(pair)) => Some(pair),
                Ok(Err(expected)) => {
                    errors.invalid_value(path, expected, val);
                    None
                }
                Err(_) => {
                    errors.type_mismatch(path, Type::String, val);
                    None
                }
            })
            .collect();
        if pairs.len() == list.len() {
            *self = pairs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;

    #[test]
    fn warn_about_every_also_pair_missing_from_pairs() {
        let old = Config::default();
        let mut new = old.clone();
        let span = |start| Span::new(start, start + 2);
        let value = Value::test_record(record! {
            "auto_pairs" => Value::test_record(record! {
                "pairs" => Value::test_list(vec![Value::test_string("()")]),
                "also" => Value::test_record(record! {
                    "in_string" => Value::test_list(vec![
                        Value::string("()", span(0)),
                        Value::string("<>", span(10)),
                    ]),
                    "in_comment" => Value::test_list(vec![
                        Value::string("[]", span(20)),
                        Value::string("<>", span(30)),
                    ]),
                }),
            }),
        });

        let result = new.update_from_value(&old, &value);

        let Ok(Some(ShellWarning::InvalidConfig { warnings })) = result else {
            panic!("expected config warnings, got: {result:?}");
        };
        let spans: Vec<_> = warnings
            .iter()
            .map(|warning| match warning {
                ConfigWarning::IncompatibleOptions { span, .. } => *span,
                other => panic!("unexpected warning: {other:?}"),
            })
            .collect();
        // `()` is in `pairs`, so it does not warn; `<>` warns once per occurrence.
        assert_eq!(spans, [span(10), span(20), span(30)]);
    }
}
