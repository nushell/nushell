use crate::completions::{Completer, CompletionOptions, MatchAlgorithm, SortBy};
use nu_engine::eval_call;
use nu_protocol::{
    ast::{Argument, Call, Expr, Expression},
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, Span, Type, Value,
};
use reedline::Suggestion;
use std::sync::Arc;

pub struct CustomCompletion {
    engine_state: Arc<EngineState>,
    stack: Stack,
    decl_id: usize,
    line: String,
    sort_by: SortBy,
}

impl CustomCompletion {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack, decl_id: usize, line: String) -> Self {
        Self {
            engine_state,
            stack,
            decl_id,
            line,
            sort_by: SortBy::None,
        }
    }

    fn map_completions<'a>(
        &self,
        list: impl Iterator<Item = &'a Value>,
        span: Span,
        offset: usize,
    ) -> Vec<Suggestion> {
        list.filter_map(move |x| {
            // Match for string values
            if let Ok(s) = x.as_string() {
                return Some(Suggestion {
                    value: s,
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: false,
                });
            }

            // Match for record values
            if let Ok((cols, vals)) = x.as_record() {
                let mut suggestion = Suggestion {
                    value: String::from(""), // Initialize with empty string
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    append_whitespace: false,
                };

                // Iterate the cols looking for `value` and `description`
                cols.iter().zip(vals).for_each(|it| {
                    // Match `value` column
                    if it.0 == "value" {
                        // Convert the value to string
                        if let Ok(val_str) = it.1.as_string() {
                            // Update the suggestion value
                            suggestion.value = val_str;
                        }
                    }

                    // Match `description` column
                    if it.0 == "description" {
                        // Convert the value to string
                        if let Ok(desc_str) = it.1.as_string() {
                            // Update the suggestion value
                            suggestion.description = Some(desc_str);
                        }
                    }
                });

                return Some(suggestion);
            }

            None
        })
        .collect()
    }
}

impl Completer for CustomCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        pos: usize,
        completion_options: &CompletionOptions,
    ) -> Vec<Suggestion> {
        // Line position
        let line_pos = pos - offset;

        // Call custom declaration
        let result = eval_call(
            &self.engine_state,
            &mut self.stack,
            &Call {
                decl_id: self.decl_id,
                head: span,
                arguments: vec![
                    Argument::Positional(Expression {
                        span: Span { start: 0, end: 0 },
                        ty: Type::String,
                        expr: Expr::String(self.line.clone()),
                        custom_completion: None,
                    }),
                    Argument::Positional(Expression {
                        span: Span { start: 0, end: 0 },
                        ty: Type::Int,
                        expr: Expr::Int(line_pos as i64),
                        custom_completion: None,
                    }),
                ],
                redirect_stdout: true,
                redirect_stderr: true,
            },
            PipelineData::new(span),
        );

        let mut custom_completion_options = None;

        // Parse result
        let suggestions = match result {
            Ok(pd) => {
                let value = pd.into_value(span);
                match &value {
                    Value::Record { .. } => {
                        let completions = value
                            .get_data_by_key("completions")
                            .and_then(|val| {
                                val.as_list()
                                    .ok()
                                    .map(|it| self.map_completions(it.iter(), span, offset))
                            })
                            .unwrap_or_default();
                        let options = value.get_data_by_key("options");

                        if let Some(Value::Record { .. }) = &options {
                            let options = options.unwrap_or_default();
                            let should_sort = options
                                .get_data_by_key("sort")
                                .and_then(|val| val.as_bool().ok())
                                .unwrap_or(false);

                            if should_sort {
                                self.sort_by = SortBy::Ascending;
                            }

                            custom_completion_options = Some(CompletionOptions {
                                case_sensitive: options
                                    .get_data_by_key("case_sensitive")
                                    .and_then(|val| val.as_bool().ok())
                                    .unwrap_or(true),
                                positional: options
                                    .get_data_by_key("positional")
                                    .and_then(|val| val.as_bool().ok())
                                    .unwrap_or(true),
                                sort_by: if should_sort {
                                    SortBy::Ascending
                                } else {
                                    SortBy::None
                                },
                                match_algorithm: match options
                                    .get_data_by_key("completion_algorithm")
                                {
                                    Some(option) => option
                                        .as_string()
                                        .ok()
                                        .and_then(|option| option.try_into().ok())
                                        .unwrap_or(MatchAlgorithm::Prefix),
                                    None => completion_options.match_algorithm,
                                },
                            });
                        }

                        completions
                    }
                    Value::List { vals, .. } => self.map_completions(vals.iter(), span, offset),
                    _ => vec![],
                }
            }
            _ => vec![],
        };

        if let Some(custom_completion_options) = custom_completion_options {
            filter(&prefix, suggestions, &custom_completion_options)
        } else {
            filter(&prefix, suggestions, completion_options)
        }
    }

    fn get_sort_by(&self) -> SortBy {
        self.sort_by
    }
}

fn filter(prefix: &[u8], items: Vec<Suggestion>, options: &CompletionOptions) -> Vec<Suggestion> {
    items
        .into_iter()
        .filter(|it| match options.match_algorithm {
            MatchAlgorithm::Prefix => match (options.case_sensitive, options.positional) {
                (true, true) => it.value.as_bytes().starts_with(prefix),
                (true, false) => it.value.contains(std::str::from_utf8(prefix).unwrap_or("")),
                (false, positional) => {
                    let value = it.value.to_lowercase();
                    let prefix = std::str::from_utf8(prefix).unwrap_or("").to_lowercase();
                    if positional {
                        value.starts_with(&prefix)
                    } else {
                        value.contains(&prefix)
                    }
                }
            },
            MatchAlgorithm::Fuzzy => options
                .match_algorithm
                .matches_u8(it.value.as_bytes(), prefix),
        })
        .collect()
}
