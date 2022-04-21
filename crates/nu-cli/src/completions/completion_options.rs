#[derive(Copy, Clone)]
pub enum SortBy {
    LevenshteinDistance,
    Ascending,
    None,
}

/// Describes how suggestions should be matched.
#[derive(Copy, Clone)]
pub enum MatchAlgorithm {
    /// Only show suggestions which begin with the given input
    ///
    /// Example:
    /// "git switch" is matched by "git sw"
    Prefix,
}

impl MatchAlgorithm {
    /// Returns whether the `needle` search text matches the given `haystack`.
    pub fn matches_str(&self, haystack: &str, needle: &str) -> bool {
        match *self {
            MatchAlgorithm::Prefix => haystack.starts_with(needle),
        }
    }

    /// Returns whether the `needle` search text matches the given `haystack`.
    pub fn matches_u8(&self, haystack: &[u8], needle: &[u8]) -> bool {
        match *self {
            MatchAlgorithm::Prefix => haystack.starts_with(needle),
        }
    }
}

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
}
