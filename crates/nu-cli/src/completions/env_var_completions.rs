use crate::completions::{
    Completer, SemanticSuggestion,
    matcher_helper::{add_semantic_suggestion, suggestion_results},
};
use nu_protocol::{
    CompletionOptions, NuMatcher, Span, SuggestionKind,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

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
            add_semantic_suggestion(
                &mut matcher,
                SemanticSuggestion {
                    suggestion: Suggestion {
                        value: name,
                        span: current_span,
                        description: None,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Value(nu_protocol::Type::String)),
                },
            );
        }

        suggestion_results(matcher)
    }
}
