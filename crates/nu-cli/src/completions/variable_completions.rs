use crate::completions::{Completer, CompletionOptions};
use nu_engine::eval_variable;
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span, Value,
};

use reedline::Suggestion;
use std::str;
use std::sync::Arc;

#[derive(Clone)]
pub struct VariableCompletion {
    engine_state: Arc<EngineState>, // TODO: Is engine state necessary? It's already a part of working set in fetch()
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
        let builtins = ["$nu", "$in", "$env", "$nothing"];
        let var_str = std::str::from_utf8(&self.var_context.0)
            .unwrap_or("")
            .to_lowercase();
        let var_id = working_set.find_variable(&self.var_context.0);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };
        let sublevels_count = self.var_context.1.len();

        // Completions for the given variable
        if !var_str.is_empty() {
            // Completion for $env.<tab>
            if var_str.as_str() == "$env" {
                let env_vars = self.stack.get_env_vars(&self.engine_state);

                // Return nested values
                if sublevels_count > 0 {
                    // Extract the target var ($env.<target-var>)
                    let target_var = self.var_context.1[0].clone();
                    let target_var_str =
                        str::from_utf8(&target_var).unwrap_or_default().to_string();

                    // Everything after the target var is the nested level ($env.<target-var>.<nested_levels>...)
                    let nested_levels: Vec<Vec<u8>> =
                        self.var_context.1.clone().into_iter().skip(1).collect();

                    if let Some(val) = env_vars.get(&target_var_str) {
                        for suggestion in
                            nested_suggestions(val.clone(), nested_levels, current_span)
                        {
                            if options
                                .match_algorithm
                                .matches_u8(suggestion.value.as_bytes(), &prefix)
                            {
                                output.push(suggestion);
                            }
                        }

                        return output;
                    }
                } else {
                    // No nesting provided, return all env vars
                    for env_var in env_vars {
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
                    for suggestion in
                        nested_suggestions(nuval, self.var_context.1.clone(), current_span)
                    {
                        if options
                            .match_algorithm
                            .matches_u8(suggestion.value.as_bytes(), &prefix)
                        {
                            output.push(suggestion);
                        }
                    }

                    return output;
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
                if let Ok(value) = var {
                    for suggestion in
                        nested_suggestions(value, self.var_context.1.clone(), current_span)
                    {
                        if options
                            .match_algorithm
                            .matches_u8(suggestion.value.as_bytes(), &prefix)
                        {
                            output.push(suggestion);
                        }
                    }

                    return output;
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

        // TODO: The following can be refactored (see find_commands_by_predicate() used in
        // command_completions).
        let mut removed_overlays = vec![];
        // Working set scope vars
        for scope_frame in working_set.delta.scope.iter().rev() {
            for overlay_frame in scope_frame
                .active_overlays(&mut removed_overlays)
                .iter()
                .rev()
            {
                for v in &overlay_frame.vars {
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
        }

        // Permanent state vars
        // for scope in &self.engine_state.scope {
        for overlay_frame in self
            .engine_state
            .active_overlays(&removed_overlays)
            .iter()
            .rev()
        {
            for v in &overlay_frame.vars {
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

        output.dedup(); // TODO: Removes only consecutive duplicates, is it intended?

        output
    }
}

// Find recursively the values for sublevels
// if no sublevels are set it returns the current value
fn nested_suggestions(
    val: Value,
    sublevels: Vec<Vec<u8>>,
    current_span: reedline::Span,
) -> Vec<Suggestion> {
    let mut output: Vec<Suggestion> = vec![];
    let value = recursive_value(val, sublevels);

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

            output
        }

        _ => output,
    }
}

// Extracts the recursive value (e.g: $var.a.b.c)
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
