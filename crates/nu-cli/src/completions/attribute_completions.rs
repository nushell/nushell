use super::{SemanticSuggestion, completion_options::NuMatcher};
use crate::completions::{Completer, Context, Fetched, to_reedline_span};
use nu_protocol::SuggestionKind;
use reedline::Suggestion;

pub struct AttributeCompletion;
pub struct AttributableCompletion;

impl Completer for AttributeCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let span = ctx.span;
        let offset = ctx.offset;
        let mut matcher = NuMatcher::new(ctx.prefix_str(), ctx.options, true);

        let attr_commands =
            working_set.find_commands_by_predicate(|s| s.starts_with(b"attr "), true);

        for (decl_id, name, desc, ty) in attr_commands {
            let name = name.strip_prefix(b"attr ").unwrap_or(&name);
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: String::from_utf8_lossy(name).into_owned(),
                    description: desc,
                    span: to_reedline_span(span, offset),
                    append_whitespace: false,
                    ..Default::default()
                },
                kind: Some(SuggestionKind::Command(ty, Some(decl_id))),
            });
        }

        Fetched::answering(matcher.suggestion_results())
    }
}

impl Completer for AttributableCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let span = ctx.span;
        let offset = ctx.offset;
        let mut matcher = NuMatcher::new(ctx.prefix_str(), ctx.options, true);

        for s in ["def", "extern", "export def", "export extern"] {
            let decl_id = working_set
                .find_decl(s.as_bytes())
                .expect("internal error, builtin declaration not found");
            let cmd = working_set.get_decl(decl_id);
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: cmd.name().into(),
                    description: Some(cmd.description().into()),
                    span: to_reedline_span(span, offset),
                    append_whitespace: true,
                    ..Default::default()
                },
                kind: Some(SuggestionKind::Command(
                    cmd.command_type(),
                    // for snippet completion in LSP
                    working_set.find_decl(s.as_bytes()),
                )),
            });
        }

        Fetched::answering(matcher.suggestion_results())
    }
}
