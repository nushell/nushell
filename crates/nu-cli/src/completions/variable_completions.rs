use crate::completions::{Completer, CompletionOptions};
use nu_engine::eval_variable;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span, Value,
};

use reedline::Suggestion;
use std::sync::Arc;

#[derive(Clone)]
pub struct VariableCompletion {
    engine_state: Arc<EngineState>,
    stack: Stack,
    var_context: (Vec<u8>, Vec<Vec<u8>>), // tuple with $var and the sublevels (.b.c.d)
}

impl VariableCompletion {
    pub fn new(
        engine_state: Arc<EngineState>,
        stack: Stack,
        var_context: (Vec<u8>, Vec<Vec<u8>>),
    ) -> Self {
        Self {
            engine_state,
            stack,
            var_context,
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
        options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        let mut output = vec![];
        let builtins = ["$nu", "$in", "$config", "$env", "$nothing"];
        let var_str = std::str::from_utf8(&self.var_context.0)
            .unwrap_or("")
            .to_lowercase();
        let var_id = working_set.find_variable(&self.var_context.0);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };

        // Completions for the given variable
        if !var_str.is_empty() {
            // Completion for $env.<tab>
            if var_str.as_str() == "$env" {
                for env_var in self.stack.get_env_vars(&self.engine_state) {
                    if options
                        .match_algorithm
                        .matches_u8(env_var.0.as_bytes(), &prefix)
                    {
                        output.push(Suggestion {
                            value: env_var.0,
                            description: None,
                            extra: None,
                            span: current_span,
                            append_whitespace: false,
                        });
                    }
                }

                return output;
            }

            // Completions for $nu.<tab>
            if var_str.as_str() == "$nu" {
                // Eval nu var
                if let Ok(nuval) = eval_variable(
                    &self.engine_state,
                    &self.stack,
                    nu_protocol::NU_VARIABLE_ID,
                    nu_protocol::Span {
                        start: current_span.start,
                        end: current_span.end,
                    },
                ) {
                    // Find recursively the values for sublevels
                    // if no sublevels are set it returns the current value
                    let value = recursive_value(nuval, self.var_context.1.clone());

                    match value {
                        Value::Record {
                            cols,
                            vals: _,
                            span: _,
                        } => {
                            // Add all the columns as completion
                            for item in cols {
                                output.push(Suggestion {
                                    value: item,
                                    description: None,
                                    extra: None,
                                    span: current_span,
                                    append_whitespace: false,
                                });
                            }

                            return output;
                        }

                        _ => {
                            return output;
                        }
                    }
                }
            }

            // Completion other variable types
            if let Some(var_id) = var_id {
                // Extract the variable value from the stack
                let var = self.stack.get_var(
                    var_id,
                    Span {
                        start: span.start,
                        end: span.end,
                    },
                );

                // If the value exists and it's of type Record
                if let Ok(mut value) = var {
                    // Find recursively the values for sublevels
                    // if no sublevels are set it returns the current value
                    value = recursive_value(value, self.var_context.1.clone());

                    match value {
                        Value::Record {
                            cols,
                            vals: _,
                            span: _,
                        } => {
                            // Add all the columns as completion
                            for item in cols {
                                output.push(Suggestion {
                                    value: item,
                                    description: None,
                                    extra: None,
                                    span: current_span,
                                    append_whitespace: false,
                                });
                            }

                            return output;
                        }

                        _ => {
                            return output;
                        }
                    }
                }
            }
        }

        // Variable completion (e.g: $en<tab> to complete $env)
        for builtin in builtins {
            if options
                .match_algorithm
                .matches_u8(builtin.as_bytes(), &prefix)
            {
                output.push(Suggestion {
                    value: builtin.to_string(),
                    description: None,
                    extra: None,
                    span: current_span,
                    append_whitespace: false,
                });
            }
        }

        // Working set scope vars
        for scope in &working_set.delta.scope {
            for v in &scope.vars {
                if options.match_algorithm.matches_u8(v.0, &prefix) {
                    output.push(Suggestion {
                        value: String::from_utf8_lossy(v.0).to_string(),
                        description: None,
                        extra: None,
                        span: current_span,
                        append_whitespace: false,
                    });
                }
            }
        }

        // Permanent state vars
        for scope in &self.engine_state.scope {
            for v in &scope.vars {
                if options.match_algorithm.matches_u8(v.0, &prefix) {
                    output.push(Suggestion {
                        value: String::from_utf8_lossy(v.0).to_string(),
                        description: None,
                        extra: None,
                        span: current_span,
                        append_whitespace: false,
                    });
                }
            }
        }

        output.dedup();

        output
    }
}

fn recursive_value(val: Value, sublevels: Vec<Vec<u8>>) -> Value {
    // Go to next sublevel
    if let Some(next_sublevel) = sublevels.clone().into_iter().next() {
        match val {
            Value::Record {
                cols,
                vals,
                span: _,
            } => {
                for item in cols.into_iter().zip(vals.into_iter()) {
                    // Check if index matches with sublevel
                    if item.0.as_bytes().to_vec() == next_sublevel {
                        // If matches try to fetch recursively the next
                        return recursive_value(item.1, sublevels.into_iter().skip(1).collect());
                    }
                }

                // Current sublevel value not found
                return Value::Nothing {
                    span: Span { start: 0, end: 0 },
                };
            }
            _ => return val,
        }
    }

    val
}
