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
    ) -> Vec<Suggestion>;

    fn get_sort_by(&self) -> SortBy {
        SortBy::Ascending
    }

    fn sort(&self, items: Vec<Suggestion>, prefix: Vec<u8>) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();
        let mut filtered_items = items;

        // Sort items
        match self.get_sort_by() {
            SortBy::LevenshteinDistance => {
                filtered_items.sort_by(|a, b| {
                    let a_distance = levenshtein_distance(&prefix_str, &a.value);
                    let b_distance = levenshtein_distance(&prefix_str, &b.value);
                    a_distance.cmp(&b_distance)
                });
            }
            SortBy::Ascending => {
                filtered_items.sort_by(|a, b| a.value.cmp(&b.value));
            }
            SortBy::None => {}
        };

        filtered_items
    }
}
