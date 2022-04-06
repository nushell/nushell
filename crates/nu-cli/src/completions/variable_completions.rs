use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};

use reedline::Suggestion;

#[derive(Clone)]
pub struct VariableCompletion {
    engine_state: EngineState,
}

impl VariableCompletion {
    pub fn new(engine_state: EngineState) -> Self {
        Self { engine_state }
    }
}

impl Completer for VariableCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        _: usize,
    ) -> (Vec<Suggestion>, CompletionOptions) {
        let mut output = vec![];

        let builtins = ["$nu", "$in", "$config", "$env", "$nothing"];

        for builtin in builtins {
            if builtin.as_bytes().starts_with(&prefix) {
                output.push(Suggestion {
                    value: builtin.to_string(),
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                });
            }
        }

        for scope in &working_set.delta.scope {
            for v in &scope.vars {
                if v.0.starts_with(&prefix) {
                    output.push(Suggestion {
                        value: String::from_utf8_lossy(v.0).to_string(),
                        description: None,
                        extra: None,
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                    });
                }
            }
        }
        for scope in &self.engine_state.scope {
            for v in &scope.vars {
                if v.0.starts_with(&prefix) {
                    output.push(Suggestion {
                        value: String::from_utf8_lossy(v.0).to_string(),
                        description: None,
                        extra: None,
                        span: reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                    });
                }
            }
        }

        output.dedup();

        (output, CompletionOptions::default())
    }
}
