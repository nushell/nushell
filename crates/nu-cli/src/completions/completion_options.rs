use std::fmt::Display;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use nu_parser::trim_quotes_str;

#[derive(Copy, Clone)]
pub enum SortBy {
    LevenshteinDistance,
    Ascending,
    None,
}

/// Describes how suggestions should be matched.
#[derive(Copy, Clone, Debug)]
pub enum MatchAlgorithm {
    /// Only show suggestions which begin with the given input
    ///
    /// Example:
    /// "git switch" is matched by "git sw"
    Prefix,

    /// Only show suggestions which contain the input chars at any place
    ///
    /// Example:
    /// "git checkout" is matched by "gco"
    Fuzzy,
}

impl MatchAlgorithm {
    /// Returns whether the `needle` search text matches the given `haystack`.
    pub fn matches_str(&self, haystack: &str, needle: &str) -> bool {
        let haystack = trim_quotes_str(haystack);
        let needle = trim_quotes_str(needle);
        match *self {
            MatchAlgorithm::Prefix => haystack.starts_with(needle),
            MatchAlgorithm::Fuzzy => {
                let matcher = SkimMatcherV2::default();
                matcher.fuzzy_match(haystack, needle).is_some()
            }
        }
    }

    /// Returns whether the `needle` search text matches the given `haystack`.
    pub fn matches_u8(&self, haystack: &[u8], needle: &[u8]) -> bool {
        match *self {
            MatchAlgorithm::Prefix => haystack.starts_with(needle),
            MatchAlgorithm::Fuzzy => {
                let haystack_str = String::from_utf8_lossy(haystack);
                let needle_str = String::from_utf8_lossy(needle);

                let matcher = SkimMatcherV2::default();
                matcher.fuzzy_match(&haystack_str, &needle_str).is_some()
            }
        }
    }
}

impl TryFrom<String> for MatchAlgorithm {
    type Error = InvalidMatchAlgorithm;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "prefix" => Ok(Self::Prefix),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err(InvalidMatchAlgorithm::Unknown),
        }
    }
}

#[derive(Debug)]
pub enum InvalidMatchAlgorithm {
    Unknown,
}

impl Display for InvalidMatchAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            InvalidMatchAlgorithm::Unknown => write!(f, "unknown match algorithm"),
        }
    }
}

impl std::error::Error for InvalidMatchAlgorithm {}

#[derive(Clone)]
pub struct CompletionOptions {
    pub case_sensitive: bool,
    pub positional: bool,
    pub sort_by: SortBy,
    pub match_algorithm: MatchAlgorithm,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            positional: true,
            sort_by: SortBy::Ascending,
            match_algorithm: MatchAlgorithm::Prefix,
        }
    }
}

#[cfg(test)]
mod test {
    use super::MatchAlgorithm;

    #[test]
    fn match_algorithm_prefix() {
        let algorithm = MatchAlgorithm::Prefix;

        assert!(algorithm.matches_str("example text", ""));
        assert!(algorithm.matches_str("example text", "examp"));
        assert!(!algorithm.matches_str("example text", "text"));

        assert!(algorithm.matches_u8(&[1, 2, 3], &[]));
        assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 2]));
        assert!(!algorithm.matches_u8(&[1, 2, 3], &[2, 3]));
    }

    #[test]
    fn match_algorithm_fuzzy() {
        let algorithm = MatchAlgorithm::Fuzzy;

        assert!(algorithm.matches_str("example text", ""));
        assert!(algorithm.matches_str("example text", "examp"));
        assert!(algorithm.matches_str("example text", "ext"));
        assert!(algorithm.matches_str("example text", "mplxt"));
        assert!(!algorithm.matches_str("example text", "mpp"));

        assert!(algorithm.matches_u8(&[1, 2, 3], &[]));
        assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 2]));
        assert!(algorithm.matches_u8(&[1, 2, 3], &[2, 3]));
        assert!(algorithm.matches_u8(&[1, 2, 3], &[1, 3]));
        assert!(!algorithm.matches_u8(&[1, 2, 3], &[2, 2]));
    }
}
