use super::{config_update_string_enum, prelude::*};
use crate as nu_protocol;
use crate::engine::Closure;

#[derive(Clone, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct XsimRomanizationConfig {
    pub enabled: bool,
    pub language_hints: Vec<String>,
}

impl Default for XsimRomanizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            language_hints: Vec::new(),
        }
    }
}

impl UpdateFromValue for XsimRomanizationConfig {
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
                "enabled" => self.enabled.update(val, path, errors),
                "language_hints" => match val {
                    Value::List { vals, .. }
                        if vals
                            .iter()
                            .all(|value| matches!(value, Value::String { .. })) =>
                    {
                        self.language_hints = vals
                            .iter()
                            .filter_map(|value| value.as_str().ok().map(str::to_owned))
                            .collect();
                    }
                    _ => errors.type_mismatch(path, Type::custom("list<string>"), val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct XsimPinyinConfig {
    pub enabled: bool,
}

impl UpdateFromValue for XsimPinyinConfig {
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
                "enabled" => self.enabled.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub struct XsimConfig {
    pub enabled: bool,
    pub romanization: XsimRomanizationConfig,
    pub pinyin: XsimPinyinConfig,
}

impl UpdateFromValue for XsimConfig {
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
                "enabled" => self.enabled.update(val, path, errors),
                "romanization" => self.romanization.update(val, path, errors),
                "pinyin" => self.pinyin.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionAlgorithm {
    #[default]
    Prefix,
    Substring,
    Fuzzy,
}

impl FromStr for CompletionAlgorithm {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "prefix" => Ok(Self::Prefix),
            "substring" => Ok(Self::Substring),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err("'prefix' or 'fuzzy' or 'substring'"),
        }
    }
}

impl UpdateFromValue for CompletionAlgorithm {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionSort {
    #[default]
    Smart,
    Alphabetical,
}

impl FromStr for CompletionSort {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "smart" => Ok(Self::Smart),
            "alphabetical" => Ok(Self::Alphabetical),
            _ => Err("'smart' or 'alphabetical'"),
        }
    }
}

impl UpdateFromValue for CompletionSort {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ExternalCompleterConfig {
    pub enable: bool,
    pub max_results: i64,
    pub completer: Option<Closure>,
}

impl Default for ExternalCompleterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            max_results: 100,
            completer: None,
        }
    }
}

impl UpdateFromValue for ExternalCompleterConfig {
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
                "completer" => match val {
                    Value::Nothing { .. } => self.completer = None,
                    Value::Closure { val, .. } => self.completer = Some(val.as_ref().clone()),
                    _ => errors.type_mismatch(path, Type::custom("closure or nothing"), val),
                },
                "max_results" => self.max_results.update(val, path, errors),
                "enable" => self.enable.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct CompletionConfig {
    pub sort: CompletionSort,
    pub case_sensitive: bool,
    pub quick: bool,
    pub partial: bool,
    pub algorithm: CompletionAlgorithm,
    pub external: ExternalCompleterConfig,
    pub use_ls_colors: bool,
    pub xsim: XsimConfig,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            sort: CompletionSort::default(),
            case_sensitive: false,
            quick: true,
            partial: true,
            algorithm: CompletionAlgorithm::default(),
            external: ExternalCompleterConfig::default(),
            use_ls_colors: true,
            xsim: XsimConfig::default(),
        }
    }
}

impl UpdateFromValue for CompletionConfig {
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
                "sort" => self.sort.update(val, path, errors),
                "quick" => self.quick.update(val, path, errors),
                "partial" => self.partial.update(val, path, errors),
                "algorithm" => self.algorithm.update(val, path, errors),
                "case_sensitive" => self.case_sensitive.update(val, path, errors),
                "external" => self.external.update(val, path, errors),
                "use_ls_colors" => self.use_ls_colors.update(val, path, errors),
                "xsim" => self.xsim.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Config, Value, record};

    use super::XsimConfig;

    #[test]
    fn xsim_defaults_are_disabled_with_romanization_ready() {
        let xsim = XsimConfig::default();

        assert!(!xsim.enabled);
        assert!(xsim.romanization.enabled);
        assert!(xsim.romanization.language_hints.is_empty());
        assert!(!xsim.pinyin.enabled);
    }

    #[test]
    fn xsim_partial_update_preserves_nested_defaults() {
        let old = Config::default();
        let mut config = old.clone();
        let value = Value::test_record(record! {
            "completions" => Value::test_record(record! {
                "xsim" => Value::test_record(record! {
                    "enabled" => Value::test_bool(true),
                    "romanization" => Value::test_record(record! {
                        "language_hints" => Value::test_list(vec![
                            Value::test_string("rus"),
                            Value::test_string("ell"),
                        ]),
                    }),
                }),
            }),
        });

        assert!(config.update_from_value(&old, &value).is_ok());
        assert!(config.completions.xsim.enabled);
        assert!(config.completions.xsim.romanization.enabled);
        assert_eq!(
            ["rus", "ell"],
            config
                .completions
                .xsim
                .romanization
                .language_hints
                .as_slice()
        );
        assert!(!config.completions.xsim.pinyin.enabled);
    }

    #[test]
    fn xsim_invalid_language_hint_list_is_rejected_atomically() {
        let mut old = Config::default();
        old.completions.xsim.romanization.language_hints = vec!["jpn".into()];
        let mut config = old.clone();
        let value = Value::test_record(record! {
            "completions" => Value::test_record(record! {
                "xsim" => Value::test_record(record! {
                    "romanization" => Value::test_record(record! {
                        "language_hints" => Value::test_list(vec![
                            Value::test_string("rus"),
                            Value::test_int(42),
                        ]),
                    }),
                }),
            }),
        });

        assert!(config.update_from_value(&old, &value).is_err());
        assert_eq!(
            ["jpn"],
            config
                .completions
                .xsim
                .romanization
                .language_hints
                .as_slice()
        );
    }
}
