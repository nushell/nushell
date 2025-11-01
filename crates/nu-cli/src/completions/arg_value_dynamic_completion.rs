use crate::completions::{
    Completer, CompletionOptions, SemanticSuggestion, completion_options::NuMatcher,
};
use nu_protocol::{
    DeclId, Span,
    engine::{ArgType, Stack, StateWorkingSet},
};
use reedline::Suggestion;

pub struct ArgValueDynamicCompletion<'a> {
    pub decl_id: DeclId,
    pub arg_type: ArgType<'a>,
}

impl<'a> Completer for ArgValueDynamicCompletion<'a> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // the `prefix` is the value of a flag
        // if user input `--foo abc`, then the `prefix` here is abc.
        // the name of flag is saved in `self.flag_name`.
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
        let mut stack = stack.to_owned();
        if let Some(items) = decl
            .get_dynamic_completion(working_set.permanent_state, &mut stack, &self.arg_type)
            .unwrap_or_default()
        {
            for i in items {
                add_suggestion(i);
            }
        }
        matcher.results()
    }
}
