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
    ) -> (Vec<Suggestion>, CompletionOptions);

    // Filter results using the completion options
    fn filter(
        &self,
        prefix: Vec<u8>,
        items: Vec<Suggestion>,
        options: CompletionOptions,
    ) -> Vec<Suggestion> {
        items
            .into_iter()
            .filter(|it| {
                // Minimise clones for new functionality
                match (options.case_sensitive, options.positional) {
                    (true, true) => it.value.as_bytes().starts_with(&prefix),
                    (true, false) => it
                        .value
                        .contains(std::str::from_utf8(&prefix).unwrap_or("")),
                    (false, positional) => {
                        let value = it.value.to_lowercase();
                        let prefix = std::str::from_utf8(&prefix).unwrap_or("").to_lowercase();
                        if positional {
                            value.starts_with(&prefix)
                        } else {
                            value.contains(&prefix)
                        }
                    }
                }
            })
            .collect()
    }

    // Sort is results using the completion options
    fn sort(
        &self,
        items: Vec<Suggestion>,
        prefix: Vec<u8>,
        options: CompletionOptions,
    ) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();
        let mut filtered_items = items;

        // Sort items
        match options.sort_by {
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
