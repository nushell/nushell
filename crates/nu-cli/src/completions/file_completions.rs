use crate::completions::{
    completion_common::{adjust_if_intermediate, complete_item, AdjustView},
    Completer, CompletionOptions,
};
use nu_ansi_term::Style;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span,
};
use nu_utils::IgnoreCaseExt;
use reedline::Suggestion;
use std::path::Path;

use super::SemanticSuggestion;

#[derive(Clone, Default)]
pub struct FileCompletion {}

impl FileCompletion {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Completer for FileCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let AdjustView {
            prefix,
            span,
            readjusted,
        } = adjust_if_intermediate(&prefix, working_set, span);

        #[allow(deprecated)]
        let items: Vec<_> = complete_item(
            readjusted,
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
                value: x.1,
                description: None,
                style: x.2,
                extra: None,
                span: reedline::Span {
                    start: x.0.start - offset,
                    end: x.0.end - offset,
                },
                append_whitespace: false,
            },
            // TODO????
            kind: None,
        })
        .collect();

        // Sort results prioritizing the non hidden folders

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

pub fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
    options: &CompletionOptions,
    engine_state: &EngineState,
    stack: &Stack,
) -> Vec<(nu_protocol::Span, String, Option<Style>)> {
    complete_item(false, span, partial, cwd, options, engine_state, stack)
}

pub fn matches(partial: &str, from: &str, options: &CompletionOptions) -> bool {
    // Check for case sensitive
    if !options.case_sensitive {
        return options
            .match_algorithm
            .matches_str(&from.to_folded_case(), &partial.to_folded_case());
    }

    options.match_algorithm.matches_str(from, partial)
}
