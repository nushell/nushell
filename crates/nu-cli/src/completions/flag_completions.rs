use crate::completions::{completion_options::NuMatcher, Completer, CompletionOptions};
use nu_protocol::{
    ast::{Expr, Expression},
    engine::{Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;

use super::SemanticSuggestion;

#[derive(Clone)]
pub struct FlagCompletion {
    expression: Expression,
}

impl FlagCompletion {
    pub fn new(expression: Expression) -> Self {
        Self { expression }
    }
}

impl Completer for FlagCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // Check if it's a flag
        if let Expr::Call(call) = &self.expression.expr {
            let decl = working_set.get_decl(call.decl_id);
            let sig = decl.signature();

            let mut matcher = NuMatcher::new(String::from_utf8_lossy(prefix), options.clone());

            for named in &sig.named {
                let flag_desc = &named.desc;
                if let Some(short) = named.short {
                    let mut named = vec![0; short.len_utf8()];
                    short.encode_utf8(&mut named);
                    named.insert(0, b'-');

                    matcher.add_semantic_suggestion(SemanticSuggestion {
                        suggestion: Suggestion {
                            value: String::from_utf8_lossy(&named).to_string(),
                            description: Some(flag_desc.to_string()),
                            span: reedline::Span {
                                start: span.start - offset,
                                end: span.end - offset,
                            },
                            append_whitespace: true,
                            ..Suggestion::default()
                        },
                        // TODO????
                        kind: None,
                    });
                }

                if named.long.is_empty() {
                    continue;
                }

                let mut named = named.long.as_bytes().to_vec();
                named.insert(0, b'-');
                named.insert(0, b'-');

                matcher.add_semantic_suggestion(SemanticSuggestion {
                    suggestion: Suggestion {
                        value: String::from_utf8_lossy(&named).to_string(),
                        description: Some(flag_desc.to_string()),
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        append_whitespace: true,
                        ..Suggestion::default()
                    },
                    // TODO????
                    kind: None,
                });
            }

            return matcher.results();
        }

        vec![]
    }
}
