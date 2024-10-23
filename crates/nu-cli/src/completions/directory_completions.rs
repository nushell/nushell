use crate::completions::{
    completion_common::{adjust_if_intermediate, complete_item, AdjustView},
    Completer, CompletionOptions,
};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::path::Path;

use super::{completion_common::FileSuggestion, SemanticSuggestion};

#[derive(Clone, Default)]
pub struct DirectoryCompletion {}

impl DirectoryCompletion {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Completer for DirectoryCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let AdjustView { prefix, span, .. } = adjust_if_intermediate(prefix, working_set, span);

        // Filter only the folders
        #[allow(deprecated)]
        let items: Vec<_> = directory_completion(
            span,
            &prefix,
            &working_set.permanent_state.current_work_dir(),
            options,
            working_set.permanent_state,
            stack,
        )
        .into_iter()
        .map(move |x| SemanticSuggestion {
            suggestion: Suggestion {
                value: x.path,
                style: x.style,
                span: reedline::Span {
                    start: x.span.start - offset,
                    end: x.span.end - offset,
                },
                ..Suggestion::default()
            },
            // TODO????
            kind: None,
        })
        .collect();

        // Separate the results between hidden and non hidden
        let mut hidden: Vec<SemanticSuggestion> = vec![];
        let mut non_hidden: Vec<SemanticSuggestion> = vec![];

        for item in items.into_iter() {
            let item_path = Path::new(&item.suggestion.value);

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
    engine_state: &EngineState,
    stack: &Stack,
) -> Vec<FileSuggestion> {
    complete_item(true, span, partial, &[cwd], options, engine_state, stack)
}
