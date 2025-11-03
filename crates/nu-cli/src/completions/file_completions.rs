use crate::completions::{
    Completer, CompletionOptions,
    completion_common::{AdjustView, adjust_if_intermediate, complete_item},
};
use nu_protocol::{
    Span,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;
use std::path::Path;

use super::{SemanticSuggestion, SuggestionKind};

pub struct FileCompletion;

impl Completer for FileCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let AdjustView {
            prefix,
            span,
            readjusted,
        } = adjust_if_intermediate(prefix.as_ref(), working_set, span);

        #[allow(deprecated)]
        let items: Vec<_> = complete_item(
            readjusted,
            span,
            &prefix,
            &[&working_set.permanent_state.current_work_dir()],
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
            kind: Some(if x.is_dir {
                SuggestionKind::Directory
            } else {
                SuggestionKind::File
            }),
        })
        .collect();

        // Sort results prioritizing the non hidden folders

        // Separate the results between hidden and non hidden
        let mut hidden: Vec<SemanticSuggestion> = vec![];
        let mut non_hidden: Vec<SemanticSuggestion> = vec![];

        for item in items.into_iter() {
            let item_path = Path::new(&item.suggestion.value);

            if let Some(value) = item_path.file_name()
                && let Some(value) = value.to_str()
            {
                if value.starts_with('.') {
                    hidden.push(item);
                } else {
                    non_hidden.push(item);
                }
            }
        }

        // Append the hidden folders to the non hidden vec to avoid creating a new vec
        non_hidden.append(&mut hidden);

        non_hidden
    }
}
