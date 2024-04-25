use crate::completions::{CompletionOptions, SortBy};
use nu_protocol::{engine::StateWorkingSet, levenshtein_distance, Span};
use reedline::Suggestion;

// Completer trait represents the three stages of the completion
// fetch, filter and sort
pub trait Completer {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion>;

    fn get_sort_by(&self) -> SortBy {
        SortBy::Ascending
    }

    fn sort(&self, items: Vec<SemanticSuggestion>, prefix: Vec<u8>) -> Vec<SemanticSuggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();
        let mut filtered_items = items;

        // Sort items
        match self.get_sort_by() {
            SortBy::LevenshteinDistance => {
                filtered_items.sort_by(|a, b| {
                    let a_distance = levenshtein_distance(&prefix_str, &a.suggestion.value);
                    let b_distance = levenshtein_distance(&prefix_str, &b.suggestion.value);
                    a_distance.cmp(&b_distance)
                });
            }
            SortBy::Ascending => {
                filtered_items.sort_by(|a, b| a.suggestion.value.cmp(&b.suggestion.value));
            }
            SortBy::None => {}
        };

        filtered_items
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct SemanticSuggestion {
    pub suggestion: Suggestion,
    pub kind: Option<SuggestionKind>,
}

// TODO: think about name: maybe suggestion context?
#[derive(Clone, Debug, PartialEq)]
pub enum SuggestionKind {
    Command(nu_protocol::engine::CommandType),
    Type(nu_protocol::Type),
}

impl From<Suggestion> for SemanticSuggestion {
    fn from(suggestion: Suggestion) -> Self {
        Self {
            suggestion,
            ..Default::default()
        }
    }
}
