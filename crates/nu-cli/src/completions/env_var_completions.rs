use crate::completions::{Completer, CompletionOptions, SemanticSuggestion};
use nu_protocol::{
    Span, SuggestionKind,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct EnvVarCompletion;

impl Completer for EnvVarCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::new(prefix, options, true);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        for name in stack.get_env_var_names(working_set.permanent_state) {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: name,
                    span: current_span,
                    description: None,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(nu_protocol::Type::String)),
            });
        }

        matcher.suggestion_results()
    }
}
