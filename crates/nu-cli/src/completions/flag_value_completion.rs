use crate::completions::{
    Completer, CompletionOptions, SemanticSuggestion, completion_options::NuMatcher,
};
use nu_protocol::{
    DeclId, Span,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

pub struct FlagValueCompletion<'a> {
    pub decl_id: DeclId,
    pub flag_name: &'a str,
}

impl<'a> Completer for FlagValueCompletion<'a> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::new(prefix, options);
        let mut add_suggestion = |value: String| {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    description: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: None,
            });
        };

        let decl = working_set.get_decl(self.decl_id);
        if let Some(items) = decl.get_completion(self.flag_name) {
            for i in items {
                add_suggestion(i);
            }
        }
        let res = matcher.results();
        res
    }
}
