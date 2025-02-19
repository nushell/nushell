use super::{completion_options::NuMatcher, SemanticSuggestion};
use crate::{
    completions::{Completer, CompletionOptions},
    SuggestionKind,
};
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;

pub struct AttributeCompletion;
pub struct AttributableCompletion;

impl Completer for AttributeCompletion {
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

        let attr_commands =
            working_set.find_commands_by_predicate(|s| s.starts_with(b"attr "), true);

        for (name, desc, ty) in attr_commands {
            let name = name.strip_prefix(b"attr ").unwrap_or(&name);
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: String::from_utf8_lossy(name).into_owned(),
                    description: desc,
                    style: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: false,
                },
                kind: Some(SuggestionKind::Command(ty)),
            });
        }

        matcher.results()
    }
}

impl Completer for AttributableCompletion {
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

        for s in ["def", "extern", "export def", "export extern"] {
            let decl_id = working_set
                .find_decl(s.as_bytes())
                .expect("internal error, builtin declaration not found");
            let cmd = working_set.get_decl(decl_id);
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: cmd.name().into(),
                    description: Some(cmd.description().into()),
                    style: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: false,
                },
                kind: Some(SuggestionKind::Command(cmd.command_type())),
            });
        }

        matcher.results()
    }
}
