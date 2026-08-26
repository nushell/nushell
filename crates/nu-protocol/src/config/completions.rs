use super::{config_update_string_enum, prelude::*};
use crate as nu_protocol;
use crate::engine::Closure;

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

/// Configured external completion closure and its optional interactive setting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExternalCompleter {
    Closure(Closure),
    Tagged { closure: Closure, interactive: bool },
}

impl ExternalCompleter {
    pub fn closure(&self) -> &Closure {
        match self {
            Self::Closure(closure) | Self::Tagged { closure, .. } => closure,
        }
    }
}

impl IntoValue for ExternalCompleter {
    fn into_value(self, span: Span) -> Value {
        match self {
            Self::Closure(closure) => closure.into_value(span),
            Self::Tagged {
                closure,
                interactive,
            } => Value::record(
                record! {
                    "closure" => closure.into_value(span),
                    "interactive" => interactive.into_value(span),
                },
                span,
            ),
        }
    }
}

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct ExternalCompleterConfig {
    pub enable: bool,
    pub max_results: i64,
    pub completer: Option<ExternalCompleter>,
    /// Run a bare `completer` inline so it can drive a picker like `fzf`.
    pub interactive: bool,
}

impl Default for ExternalCompleterConfig {
    fn default() -> Self {
        Self {
            enable: true,
            max_results: 100,
            completer: None,
            interactive: false,
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
                    Value::Closure { val, .. } => {
                        self.completer = Some(ExternalCompleter::Closure(val.as_ref().clone()))
                    }
                    Value::Record { val: fields, .. } => {
                        match (fields.get("closure"), fields.get("interactive")) {
                            (Some(Value::Closure { val: closure, .. }), interactive) => {
                                let interactive =
                                    interactive.and_then(|v| v.as_bool().ok()).unwrap_or(false);
                                self.completer = Some(ExternalCompleter::Tagged {
                                    closure: closure.as_ref().clone(),
                                    interactive,
                                });
                            }
                            _ => errors.type_mismatch(
                                path,
                                Type::custom("{closure: closure, interactive?: bool}"),
                                val,
                            ),
                        }
                    }
                    _ => errors.type_mismatch(
                        path,
                        Type::custom("closure, {closure, interactive}, or nothing"),
                        val,
                    ),
                },
                "max_results" => self.max_results.update(val, path, errors),
                "enable" => self.enable.update(val, path, errors),
                "interactive" => self.interactive.update(val, path, errors),
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
    /// Completion results the cache keeps before evicting the least recently used entry; `0` disables it.
    pub cache_size: i64,
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
            cache_size: 100,
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
                "cache_size" => self.cache_size.update(val, path, errors),
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
