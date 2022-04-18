use crate::completions::{Completer, CompletionOptions, SortBy};
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
}

impl CustomCompletion {
    pub fn new(engine_state: Arc<EngineState>, stack: Stack, decl_id: usize, line: String) -> Self {
        Self {
            engine_state,
            stack,
            decl_id,
            line,
        }
    }

    fn map_completions<'a>(
        &self,
        list: impl Iterator<Item = &'a Value>,
        span: Span,
        offset: usize,
    ) -> Vec<Suggestion> {
        list.filter_map(move |x| {
            let s = x.as_string();

            match s {
                Ok(s) => Some(Suggestion {
                    value: s,
                    description: None,
                    extra: None,
                    span: reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                }),
                Err(_) => None,
            }
        })
        .collect()
    }
}

impl Completer for CustomCompletion {
    fn fetch(
        &mut self,
        _: &StateWorkingSet,
        _: Vec<u8>,
        span: Span,
        offset: usize,
        pos: usize,
    ) -> (Vec<Suggestion>, CompletionOptions) {
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

        // Parse result
        let (suggestions, options) = match result {
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

                        let options = if let Some(Value::Record { .. }) = &options {
                            let options = options.unwrap_or_default();
                            let should_sort = options
                                .get_data_by_key("sort")
                                .and_then(|val| val.as_bool().ok())
                                .unwrap_or(false);

                            CompletionOptions {
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
                            }
                        } else {
                            CompletionOptions::default()
                        };

                        (completions, options)
                    }
                    Value::List { vals, .. } => {
                        let completions = self.map_completions(vals.iter(), span, offset);
                        (completions, CompletionOptions::default())
                    }
                    _ => (vec![], CompletionOptions::default()),
                }
            }
            _ => (vec![], CompletionOptions::default()),
        };

        (suggestions, options)
    }
}
