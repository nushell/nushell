use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
    Span,
};

use reedline::Suggestion;

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
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        // Check if it's a flag
        if let Expr::Call(call) = &self.expression.expr {
            let decl = working_set.get_decl(call.decl_id);
            let sig = decl.signature();

            let mut output = vec![];

            for named in &sig.named {
                let flag_desc = &named.desc;
                if let Some(short) = named.short {
                    let mut named = vec![0; short.len_utf8()];
                    short.encode_utf8(&mut named);
                    named.insert(0, b'-');

                    if options.match_algorithm.matches_u8(&named, &prefix) {
                        output.push(Suggestion {
                            value: String::from_utf8_lossy(&named).to_string(),
                            description: Some(flag_desc.to_string()),
                            extra: None,
                            span: reedline::Span {
                                start: span.start - offset,
                                end: span.end - offset,
                            },
                            append_whitespace: true,
                        });
                    }
                }

                if named.long.is_empty() {
                    continue;
                }

                let mut named = named.long.as_bytes().to_vec();
                named.insert(0, b'-');
                named.insert(0, b'-');

                if options.match_algorithm.matches_u8(&named, &prefix) {
                    output.push(Suggestion {
                        value: String::from_utf8_lossy(&named).to_string(),
                        description: Some(flag_desc.to_string()),
                        extra: None,
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        append_whitespace: true,
                    });
                }
            }

            return output;
        }

        vec![]
    }
}
