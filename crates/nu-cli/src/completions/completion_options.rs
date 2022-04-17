#[derive(Copy, Clone)]
pub enum SortBy {
    LevenshteinDistance,
    Ascending,
    None,
}

#[derive(Clone)]
pub struct CompletionOptions {
    pub case_sensitive: bool,
    pub positional: bool,
    pub sort_by: SortBy,
}

impl CompletionOptions {
    pub fn new(case_sensitive: bool, positional: bool, sort_by: SortBy) -> Self {
        Self {
            case_sensitive,
            positional,
            sort_by,
        }
    }
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            positional: true,
            sort_by: SortBy::Ascending,
        }
    }
}
