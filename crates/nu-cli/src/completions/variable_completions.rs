use crate::completions::{Completer, CompletionOptions};
use nu_engine::{column::get_columns, eval_variable};
use nu_protocol::{
    engine::{EngineState, Stack, StateWorkingSet},
    Span, Value,
};

use reedline::Suggestion;
use std::str;
use std::sync::Arc;

use super::MatchAlgorithm;

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
        let builtins = ["$nu", "$in", "$env"];
        let var_str = std::str::from_utf8(&self.var_context.0).unwrap_or("");
        let var_id = working_set.find_variable(&self.var_context.0);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };
        let sublevels_count = self.var_context.1.len();

        // Completions for the given variable
        if !var_str.is_empty() {
            // Completion for $env.<tab>
            if var_str == "$env" {
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
                            if options.match_algorithm.matches_u8_insensitive(
                                options.case_sensitive,
                                suggestion.value.as_bytes(),
                                &prefix,
                            ) {
                                output.push(suggestion);
                            }
                        }

                        return output;
                    }
                } else {
                    // No nesting provided, return all env vars
                    for env_var in env_vars {
                        if options.match_algorithm.matches_u8_insensitive(
                            options.case_sensitive,
                            env_var.0.as_bytes(),
                            &prefix,
                        ) {
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
            if var_str == "$nu" {
                // Eval nu var
                if let Ok(nuval) = eval_variable(
                    &self.engine_state,
                    &self.stack,
                    nu_protocol::NU_VARIABLE_ID,
                    nu_protocol::Span::new(current_span.start, current_span.end),
                ) {
                    for suggestion in
                        nested_suggestions(nuval, self.var_context.1.clone(), current_span)
                    {
                        if options.match_algorithm.matches_u8_insensitive(
                            options.case_sensitive,
                            suggestion.value.as_bytes(),
                            &prefix,
                        ) {
                            output.push(suggestion);
                        }
                    }

                    return output;
                }
            }

            // Completion other variable types
            if let Some(var_id) = var_id {
                // Extract the variable value from the stack
                let var = self.stack.get_var(var_id, Span::new(span.start, span.end));

                // If the value exists and it's of type Record
                if let Ok(value) = var {
                    for suggestion in
                        nested_suggestions(value, self.var_context.1.clone(), current_span)
                    {
                        if options.match_algorithm.matches_u8_insensitive(
                            options.case_sensitive,
                            suggestion.value.as_bytes(),
                            &prefix,
                        ) {
                            output.push(suggestion);
                        }
                    }

                    return output;
                }
            }
        }

        // Variable completion (e.g: $en<tab> to complete $env)
        for builtin in builtins {
            if options.match_algorithm.matches_u8_insensitive(
                options.case_sensitive,
                builtin.as_bytes(),
                &prefix,
            ) {
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
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                for v in &overlay_frame.vars {
                    if options.match_algorithm.matches_u8_insensitive(
                        options.case_sensitive,
                        v.0,
                        &prefix,
                    ) {
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
        for overlay_frame in self.engine_state.active_overlays(&removed_overlays).rev() {
            for v in &overlay_frame.vars {
                if options.match_algorithm.matches_u8_insensitive(
                    options.case_sensitive,
                    v.0,
                    &prefix,
                ) {
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
        Value::Record { val, .. } => {
            // Add all the columns as completion
            for (col, _) in val.into_iter() {
                output.push(Suggestion {
                    value: col,
                    description: None,
                    extra: None,
                    span: current_span,
                    append_whitespace: false,
                });
            }

            output
        }
        Value::LazyRecord { val, .. } => {
            // Add all the columns as completion
            for column_name in val.column_names() {
                output.push(Suggestion {
                    value: column_name.to_string(),
                    description: None,
                    extra: None,
                    span: current_span,
                    append_whitespace: false,
                });
            }

            output
        }
        Value::List { vals, .. } => {
            for column_name in get_columns(vals.as_slice()) {
                output.push(Suggestion {
                    value: column_name,
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
        let span = val.span();
        match val {
            Value::Record { val, .. } => {
                for item in val {
                    // Check if index matches with sublevel
                    if item.0.as_bytes().to_vec() == next_sublevel {
                        // If matches try to fetch recursively the next
                        return recursive_value(item.1, sublevels.into_iter().skip(1).collect());
                    }
                }

                // Current sublevel value not found
                return Value::nothing(span);
            }
            Value::LazyRecord { val, .. } => {
                for col in val.column_names() {
                    if col.as_bytes().to_vec() == next_sublevel {
                        return recursive_value(
                            val.get_column_value(col).unwrap_or_default(),
                            sublevels.into_iter().skip(1).collect(),
                        );
                    }
                }

                // Current sublevel value not found
                return Value::nothing(span);
            }
            Value::List { vals, .. } => {
                for col in get_columns(vals.as_slice()) {
                    if col.as_bytes().to_vec() == next_sublevel {
                        return recursive_value(
                            Value::list(vals, span)
                                .get_data_by_key(&col)
                                .unwrap_or_default(),
                            sublevels.into_iter().skip(1).collect(),
                        );
                    }
                }

                // Current sublevel value not found
                return Value::nothing(span);
            }
            _ => return val,
        }
    }

    val
}

impl MatchAlgorithm {
    pub fn matches_u8_insensitive(&self, sensitive: bool, haystack: &[u8], needle: &[u8]) -> bool {
        if sensitive {
            self.matches_u8(haystack, needle)
        } else {
            self.matches_u8(&haystack.to_ascii_lowercase(), &needle.to_ascii_lowercase())
        }
    }
}
