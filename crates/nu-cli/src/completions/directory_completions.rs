use crate::completions::{file_path_completion, Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    levenshtein_distance, Span,
};
use reedline::Suggestion;
use std::path::Path;
use std::sync::Arc;

const SEP: char = std::path::MAIN_SEPARATOR;

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
        _: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
            match d.as_string() {
                Ok(s) => s,
                Err(_) => "".to_string(),
            }
        } else {
            "".to_string()
        };
        let partial = String::from_utf8_lossy(&prefix).to_string();

        // Filter only the folders
        let output: Vec<_> = file_path_completion(span, &partial, &cwd, options.match_algorithm)
            .into_iter()
            .filter_map(move |x| {
                if x.1.ends_with(SEP) {
                    return Some(Suggestion {
                        value: x.1,
                        description: None,
                        extra: None,
                        span: reedline::Span {
                            start: x.0.start - offset,
                            end: x.0.end - offset,
                        },
                    });
                }

                None
            })
            .collect();

        output
    }

    // Sort results prioritizing the non hidden folders
    fn sort(&self, items: Vec<Suggestion>, prefix: Vec<u8>) -> Vec<Suggestion> {
        let prefix_str = String::from_utf8_lossy(&prefix).to_string();

        // Sort items
        let mut sorted_items = items;
        sorted_items.sort_by(|a, b| a.value.cmp(&b.value));
        sorted_items.sort_by(|a, b| {
            let a_distance = levenshtein_distance(&prefix_str, &a.value);
            let b_distance = levenshtein_distance(&prefix_str, &b.value);
            a_distance.cmp(&b_distance)
        });

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
