use crate::completions::{completion_common::complete_item, Completer, CompletionOptions, SortBy};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    levenshtein_distance, Span,
};
use reedline::Suggestion;
use std::path::{is_separator, Path, MAIN_SEPARATOR as SEP};
use std::sync::Arc;

#[derive(Clone)]
pub struct DirectoryCompletion {
    engine_state: Arc<EngineState>,
}

impl DirectoryCompletion {
    pub fn new(engine_state: Arc<EngineState>) -> Self {
        Self { engine_state }
    }
}

impl Completer for DirectoryCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let span_contents =
            String::from_utf8_lossy(working_set.get_span_contents(span)).to_string();
        let mut prefix = String::from_utf8_lossy(&prefix).to_string();
        let mut end = span.end;

        let cursor_size = if cfg!(target_os = "windows") { 0 } else { 1 };
        if span_contents.chars().count() - prefix.chars().count() > cursor_size {
            let remnant: String = span_contents
                .chars()
                .skip(prefix.chars().count() + cursor_size)
                .take_while(|&c| !is_separator(c))
                .collect();
            prefix.push_str(&remnant);
            end = span.start + prefix.len() + cursor_size;
        };

        // Filter only the folders
        let output: Vec<_> = directory_completion(
            Span::new(span.start, end),
            &prefix,
            &self.engine_state.current_work_dir(),
            options,
        )
        .into_iter()
        .map(move |x| Suggestion {
            value: x.1,
            description: None,
            extra: None,
            span: reedline::Span {
                start: x.0.start - offset,
                end: x.0.end - offset,
            },
            append_whitespace: false,
        })
        .collect();

        output
    }

    // Sort results prioritizing the non hidden folders
    fn sort(&self, items: Vec<Suggestion>, prefix: Vec<u8>) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();

        // Sort items
        let mut sorted_items = items;

        match self.get_sort_by() {
            SortBy::Ascending => {
                sorted_items.sort_by(|a, b| {
                    // Ignore trailing slashes in folder names when sorting
                    a.value
                        .trim_end_matches(SEP)
                        .cmp(b.value.trim_end_matches(SEP))
                });
            }
            SortBy::LevenshteinDistance => {
                sorted_items.sort_by(|a, b| {
                    let a_distance = levenshtein_distance(&prefix_str, &a.value);
                    let b_distance = levenshtein_distance(&prefix_str, &b.value);
                    a_distance.cmp(&b_distance)
                });
            }
            _ => (),
        }

        // Separate the results between hidden and non hidden
        let mut hidden: Vec<Suggestion> = vec![];
        let mut non_hidden: Vec<Suggestion> = vec![];

        for item in sorted_items.into_iter() {
            let item_path = Path::new(&item.value);

            if let Some(value) = item_path.file_name() {
                if let Some(value) = value.to_str() {
                    if value.starts_with('.') {
                        hidden.push(item);
                    } else {
                        non_hidden.push(item);
                    }
                }
            }
        }

        // Append the hidden folders to the non hidden vec to avoid creating a new vec
        non_hidden.append(&mut hidden);

        non_hidden
    }
}

pub fn directory_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
) -> Vec<(nu_protocol::Span, String)> {
    complete_item(true, span, partial, cwd, options)
}
