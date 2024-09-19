use super::prelude::*;
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

#[derive(Clone, Debug, IntoValue, Serialize, Deserialize)]
pub struct CompleterConfig {
    pub sort: CompletionSort,
    pub case_sensitive: bool,
    pub quick: bool,
    pub partial: bool,
    pub algorithm: CompletionAlgorithm,
    pub external: ExternalCompleterConfig,
    pub use_ls_colors: bool,
}

impl Default for CompleterConfig {
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
