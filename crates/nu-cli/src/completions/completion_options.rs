#[derive(Copy, Clone)]
pub enum SortBy {
    LevenshteinDistance,
    Ascending,
    None,
}

#[derive(Copy, Clone)]
pub enum MatchAlgorithm {
    Prefix,
}

impl MatchAlgorithm {
    pub fn matches_str(&self, haystack: &str, needle: &str) -> bool {
        match *self {
            MatchAlgorithm::Prefix => haystack.starts_with(needle),
        }
    }

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
