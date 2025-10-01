use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_protocol::{
    Span,
    engine::{Stack, StateWorkingSet},
};
use nu_utils::NuCow;
use reedline::Suggestion;

use super::completion_options::NuMatcher;

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
        let mut matcher = NuMatcher::new(prefix, options);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        let mut add_suggestion = |option: &str| {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: option.to_owned(),
                    span: current_span,
                    description: None,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(nu_protocol::Type::String)),
            });
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

        matcher.results()
    }
}
