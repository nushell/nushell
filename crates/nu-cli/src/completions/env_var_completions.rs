use crate::completions::{Completer, Context, Fetched, SemanticSuggestion, to_reedline_span};
use nu_protocol::SuggestionKind;
use reedline::Suggestion;

use super::completion_options::NuMatcher;

pub struct EnvVarCompletion;

impl Completer for EnvVarCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let stack = ctx.stack;
        let mut matcher = NuMatcher::new(ctx.prefix_str(), ctx.options, true);
        let current_span = to_reedline_span(ctx.span, ctx.offset);

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

        Fetched::answering(matcher.suggestion_results())
    }
}
