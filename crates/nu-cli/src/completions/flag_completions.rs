use crate::completions::{
    completion_options::{MatcherOptions, NuMatcher},
    Completer, CompletionOptions,
};
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
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // Check if it's a flag
        if let Expr::Call(call) = &self.expression.expr {
            let decl = working_set.get_decl(call.decl_id);
            let sig = decl.signature();

            let prefix = String::from_utf8_lossy(&prefix);
            let mut matcher = NuMatcher::new(
                prefix,
                MatcherOptions::new(options).sort_by(self.get_sort_by()),
            );

            for named in &sig.named {
                let flag_desc = &named.desc;
                if let Some(short) = named.short {
                    let named = format!("-{}", short);

                    matcher.add(
                        named.clone(),
                        SemanticSuggestion {
                            suggestion: Suggestion {
                                value: named,
                                description: Some(flag_desc.to_string()),
                                style: None,
                                extra: None,
                                span: reedline::Span {
                                    start: span.start - offset,
                                    end: span.end - offset,
                                },
                                append_whitespace: true,
                            },
                            // TODO????
                            kind: None,
                        },
                    );
                }

                if named.long.is_empty() {
                    continue;
                }

                let named = format!("--{}", named.long);

                matcher.add(
                    named.clone(),
                    SemanticSuggestion {
                        suggestion: Suggestion {
                            value: named,
                            description: Some(flag_desc.to_string()),
                            style: None,
                            extra: None,
                            span: reedline::Span {
                                start: span.start - offset,
                                end: span.end - offset,
                            },
                            append_whitespace: true,
                        },
                        // TODO????
                        kind: None,
                    },
                );
            }

            panic!("{:?}", matcher.get_results());
        }

        vec![]
    }
}
