use crate::completions::{
    Completer, SemanticSuggestion,
    matcher_helper::{add_semantic_suggestion, suggestion_results},
};
use nu_protocol::{
    CompletionOptions, NuMatcher, Span, SuggestionKind,
    engine::{Stack, StateWorkingSet},
};
use nu_utils::NuCow;
use reedline::Suggestion;

pub struct StaticCompletion {
    options: NuCow<&'static [&'static str], Vec<String>>,
}

impl StaticCompletion {
    pub fn new(options: NuCow<&'static [&'static str], Vec<String>>) -> Self {
        Self { options }
    }
}

impl Completer for StaticCompletion {
    fn fetch(
        &mut self,
        _working_set: &StateWorkingSet,
        _stack: &Stack,
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

        let mut add_suggestion = |option: &str| {
            add_semantic_suggestion(
                &mut matcher,
                SemanticSuggestion {
                    suggestion: Suggestion {
                        value: option.to_owned(),
                        span: current_span,
                        description: None,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Value(nu_protocol::Type::String)),
                },
            );
        };

        match self.options {
            NuCow::Borrowed(b) => {
                for &option in b {
                    add_suggestion(option);
                }
            }
            NuCow::Owned(ref o) => {
                for option in o {
                    add_suggestion(option.as_str());
                }
            }
        }

        suggestion_results(matcher)
    }
}
