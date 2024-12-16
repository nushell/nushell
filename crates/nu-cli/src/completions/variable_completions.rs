use crate::completions::{Completer, CompletionOptions, SemanticSuggestion, SuggestionKind};
use nu_engine::{column::get_columns, eval_variable};
#[cfg(windows)]
use nu_protocol::engine::is_env_var_for_drive;
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span, Value,
};
use reedline::Suggestion;
use std::str;

use super::completion_options::NuMatcher;

#[derive(Clone)]
pub struct VariableCompletion {
    var_context: (Vec<u8>, Vec<Vec<u8>>), // tuple with $var and the sublevels (.b.c.d)
}

impl VariableCompletion {
    pub fn new(var_context: (Vec<u8>, Vec<Vec<u8>>)) -> Self {
        Self { var_context }
    }
}

impl Completer for VariableCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let builtins = ["$nu", "$in", "$env"];
        let var_str = std::str::from_utf8(&self.var_context.0).unwrap_or("");
        let var_id = working_set.find_variable(&self.var_context.0);
        let current_span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };
        let sublevels_count = self.var_context.1.len();
        let prefix_str = String::from_utf8_lossy(prefix);
        let mut matcher = NuMatcher::new(prefix_str, options.clone());

        // Completions for the given variable
        if !var_str.is_empty() {
            // Completion for $env.<tab>
            if var_str == "$env" {
                #[cfg(not(windows))]
                let env_vars = stack.get_env_vars(working_set.permanent_state);
                #[cfg(windows)]
                let env_vars = {
                    let mut env_vars = stack.get_env_vars(working_set.permanent_state);
                    // Hide env_vars for PWD-per-drive from being listed in completion selection
                    env_vars.retain(|key, _| !is_env_var_for_drive(key));
                    env_vars
                };

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
                        for suggestion in nested_suggestions(val, &nested_levels, current_span) {
                            matcher.add_semantic_suggestion(suggestion);
                        }

                        return matcher.results();
                    }
                } else {
                    // No nesting provided, return all env vars
                    for env_var in env_vars {
                        matcher.add_semantic_suggestion(SemanticSuggestion {
                            suggestion: Suggestion {
                                value: env_var.0,
                                span: current_span,
                                ..Suggestion::default()
                            },
                            kind: Some(SuggestionKind::Type(env_var.1.get_type())),
                        });
                    }

                    return matcher.results();
                }
            }

            // Completions for $nu.<tab>
            if var_str == "$nu" {
                // Eval nu var
                if let Ok(nuval) = eval_variable(
                    working_set.permanent_state,
                    stack,
                    nu_protocol::NU_VARIABLE_ID,
                    nu_protocol::Span::new(current_span.start, current_span.end),
                ) {
                    for suggestion in nested_suggestions(&nuval, &self.var_context.1, current_span)
                    {
                        matcher.add_semantic_suggestion(suggestion);
                    }

                    return matcher.results();
                }
            }

            // Completion other variable types
            if let Some(var_id) = var_id {
                // Extract the variable value from the stack
                let var = stack.get_var(var_id, Span::new(span.start, span.end));

                // If the value exists and it's of type Record
                if let Ok(value) = var {
                    for suggestion in nested_suggestions(&value, &self.var_context.1, current_span)
                    {
                        matcher.add_semantic_suggestion(suggestion);
                    }

                    return matcher.results();
                }
            }
        }

        // Variable completion (e.g: $en<tab> to complete $env)
        for builtin in builtins {
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value: builtin.to_string(),
                    span: current_span,
                    ..Suggestion::default()
                },
                // TODO is there a way to get the VarId to get the type???
                kind: None,
            });
        }

        // TODO: The following can be refactored (see find_commands_by_predicate() used in
        // command_completions).
        let mut removed_overlays = vec![];
        // Working set scope vars
        for scope_frame in working_set.delta.scope.iter().rev() {
            for overlay_frame in scope_frame.active_overlays(&mut removed_overlays).rev() {
                for v in &overlay_frame.vars {
                    matcher.add_semantic_suggestion(SemanticSuggestion {
                        suggestion: Suggestion {
                            value: String::from_utf8_lossy(v.0).to_string(),
                            span: current_span,
                            ..Suggestion::default()
                        },
                        kind: Some(SuggestionKind::Type(
                            working_set.get_variable(*v.1).ty.clone(),
                        )),
                    });
                }
            }
        }

        // Permanent state vars
        // for scope in &self.engine_state.scope {
        for overlay_frame in working_set
            .permanent_state
            .active_overlays(&removed_overlays)
            .rev()
        {
            for v in &overlay_frame.vars {
                matcher.add_semantic_suggestion(SemanticSuggestion {
                    suggestion: Suggestion {
                        value: String::from_utf8_lossy(v.0).to_string(),
                        span: current_span,
                        ..Suggestion::default()
                    },
                    kind: Some(SuggestionKind::Type(
                        working_set.get_variable(*v.1).ty.clone(),
                    )),
                });
            }
        }

        matcher.results()
    }
}

// Find recursively the values for sublevels
// if no sublevels are set it returns the current value
fn nested_suggestions(
    val: &Value,
    sublevels: &[Vec<u8>],
    current_span: reedline::Span,
) -> Vec<SemanticSuggestion> {
    let mut output: Vec<SemanticSuggestion> = vec![];
    let value = recursive_value(val, sublevels).unwrap_or_else(Value::nothing);

    let kind = SuggestionKind::Type(value.get_type());
    match value {
        Value::Record { val, .. } => {
            // Add all the columns as completion
            for col in val.columns() {
                output.push(SemanticSuggestion {
                    suggestion: Suggestion {
                        value: col.clone(),
                        span: current_span,
                        ..Suggestion::default()
                    },
                    kind: Some(kind.clone()),
                });
            }

            output
        }
        Value::List { vals, .. } => {
            for column_name in get_columns(vals.as_slice()) {
                output.push(SemanticSuggestion {
                    suggestion: Suggestion {
                        value: column_name,
                        span: current_span,
                        ..Suggestion::default()
                    },
                    kind: Some(kind.clone()),
                });
            }

            output
        }
        _ => output,
    }
}

// Extracts the recursive value (e.g: $var.a.b.c)
fn recursive_value(val: &Value, sublevels: &[Vec<u8>]) -> Result<Value, Span> {
    // Go to next sublevel
    if let Some((sublevel, next_sublevels)) = sublevels.split_first() {
        let span = val.span();
        match val {
            Value::Record { val, .. } => {
                if let Some((_, value)) = val.iter().find(|(key, _)| key.as_bytes() == sublevel) {
                    // If matches try to fetch recursively the next
                    recursive_value(value, next_sublevels)
                } else {
                    // Current sublevel value not found
                    Err(span)
                }
            }
            Value::List { vals, .. } => {
                for col in get_columns(vals.as_slice()) {
                    if col.as_bytes() == *sublevel {
                        let val = val.get_data_by_key(&col).ok_or(span)?;
                        return recursive_value(&val, next_sublevels);
                    }
                }

                // Current sublevel value not found
                Err(span)
            }
            _ => Ok(val.clone()),
        }
    } else {
        Ok(val.clone())
    }
}
