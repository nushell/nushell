use crate::completions::{
    Completer, Context, Fetched, SemanticSuggestion, completion_options::NuMatcher,
    to_reedline_span,
};
use nu_protocol::{DeclId, SuggestionKind};
use reedline::Suggestion;

#[derive(Clone)]
pub struct FlagCompletion {
    pub decl_id: DeclId,
}

impl Completer for FlagCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let span = ctx.span;
        let offset = ctx.offset;
        let mut matcher = NuMatcher::new(ctx.prefix_str(), ctx.options, true);
        let mut add_suggestion = |value: String, description: String| {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    description: Some(description),
                    span: to_reedline_span(span, offset),
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Flag),
            });
        };

        let decl = working_set.get_decl(self.decl_id);
        let sig = decl.signature();
        for named in &sig.named {
            if let Some(short) = named.short {
                let mut name = String::from("-");
                name.push(short);
                add_suggestion(name, named.desc.clone());
            }

            if let Some(long) = named.long_name() {
                add_suggestion(format!("--{long}"), named.desc.clone());
            }
        }
        Fetched::answering(matcher.suggestion_results())
    }
}
