use crate::completions::{
    Completer, SemanticSuggestion,
    matcher_helper::{add_semantic_suggestion, suggestion_results},
};
use nu_protocol::{
    CompletionOptions, DeclId, NuMatcher, Span, SuggestionKind,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

#[derive(Clone)]
pub struct FlagCompletion {
    pub decl_id: DeclId,
}

impl Completer for FlagCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::new(prefix, options, true);
        let mut add_suggestion = |value: String, description: String| {
            add_semantic_suggestion(
                &mut matcher,
                SemanticSuggestion {
                    suggestion: Suggestion {
                        value,
                        description: Some(description),
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        append_whitespace: true,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Flag),
                },
            );
        };

        let decl = working_set.get_decl(self.decl_id);
        let sig = decl.signature();
        for named in &sig.named {
            if let Some(short) = named.short {
                let mut name = String::from("-");
                name.push(short);
                add_suggestion(name, named.desc.clone());
            }

            if named.long.is_empty() {
                continue;
            }
            add_suggestion(format!("--{}", named.long), named.desc.clone());
        }
        suggestion_results(matcher)
    }
}
