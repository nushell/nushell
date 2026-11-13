use crate::completions::{Completer, Context, Fetched, SemanticSuggestion, to_reedline_span};
use nu_protocol::SuggestionKind;
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
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let mut matcher = NuMatcher::new(ctx.prefix_str(), ctx.options, true);
        let current_span = to_reedline_span(ctx.span, ctx.offset);

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

        Fetched::answering(matcher.suggestion_results())
    }
}
