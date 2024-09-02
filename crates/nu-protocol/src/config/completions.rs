use super::{
    config_update_string_enum, prelude::*, report_invalid_config_key, report_invalid_config_value,
};
use crate as nu_protocol;
use crate::engine::Closure;

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionAlgorithm {
    #[default]
    Prefix,
    Fuzzy,
}

impl FromStr for CompletionAlgorithm {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "prefix" => Ok(Self::Prefix),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err("expected either 'prefix' or 'fuzzy'"),
        }
    }
}

impl UpdateFromValue for CompletionAlgorithm {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
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
            _ => Err("expected either 'smart' or 'alphabetical'"),
        }
    }
}

impl UpdateFromValue for CompletionSort {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
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
        errors: &mut Vec<ShellError>,
    ) {
        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "completer" => match val {
                    Value::Nothing { .. } => self.completer = None,
                    Value::Closure { val, .. } => self.completer = Some(val.as_ref().clone()),
                    _ => report_invalid_config_value(
                        "should be a closure or null",
                        span,
                        path,
                        errors,
                    ),
                },
                "max_results" => self.max_results.update(val, path, errors),
                "enable" => self.enable.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
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
        }
    }
}

impl UpdateFromValue for CompletionConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut Vec<ShellError>,
    ) {
        let span = value.span();
        let Value::Record { val: record, .. } = value else {
            report_invalid_config_value("should be a record", span, path, errors);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            let span = val.span();
            match col.as_str() {
                "sort" => self.sort.update(val, path, errors),
                "quick" => self.quick.update(val, path, errors),
                "partial" => self.partial.update(val, path, errors),
                "algorithm" => self.algorithm.update(val, path, errors),
                "case_sensitive" => self.case_sensitive.update(val, path, errors),
                "external" => self.external.update(val, path, errors),
                "use_ls_colors" => self.use_ls_colors.update(val, path, errors),
                _ => report_invalid_config_key(span, path, errors),
            }
        }
    }
}
