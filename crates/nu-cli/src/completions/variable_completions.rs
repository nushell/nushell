use crate::completions::{Completer, CompletionOptions};
use nu_protocol::{
    engine::{EngineState, StateWorkingSet},
    Span,
};
use reedline::Suggestion;
use std::sync::Arc;

#[derive(Clone)]
pub struct VariableCompletion {
    engine_state: Arc<EngineState>,
    previous_expr: Vec<u8>,
}

impl VariableCompletion {
    pub fn new(engine_state: Arc<EngineState>, previous_expr: Vec<u8>) -> Self {
        Self {
            engine_state,
            previous_expr,
        }
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
        let previous_expr_str = std::str::from_utf8(&self.previous_expr)
            .unwrap_or("")
            .to_lowercase();

        // Completions for the given variable (e.g: $env.<tab> for completing $env.SOMETHING)
        if !self.previous_expr.is_empty() && previous_expr_str.as_str() == "$env" {
            for env_var in working_set.list_env() {
                output.push(Suggestion {
                    value: env_var,
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                });
            }

            return (output, CompletionOptions::default());
        }

        for builtin in builtins {
            // Variable completion (e.g: $en<tab> to complete $env)
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

        // Working set scope vars
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

        // Permanent state vars
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
