use nu_engine::eval_block;
use nu_protocol::{
    engine::{EngineState, Stack},
    IntoPipelineData, Span, Value,
};
use reedline::{menu_functions::parse_selection_char, Completer, Suggestion};
use std::sync::Arc;

const SELECTION_CHAR: char = '!';

pub struct NuMenuCompleter {
    block_id: usize,
    span: Span,
    stack: Stack,
    engine_state: Arc<EngineState>,
    only_buffer_difference: bool,
}

impl NuMenuCompleter {
    pub fn new(
        block_id: usize,
        span: Span,
        stack: Stack,
        engine_state: Arc<EngineState>,
        only_buffer_difference: bool,
    ) -> Self {
        Self {
            block_id,
            span,
            stack,
            engine_state,
            only_buffer_difference,
        }
    }
}

impl Completer for NuMenuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let parsed = parse_selection_char(line, SELECTION_CHAR);

        let block = self.engine_state.get_block(self.block_id);

        if let Some(buffer) = block.signature.get_positional(0) {
            if let Some(buffer_id) = &buffer.var_id {
                let line_buffer = Value::String {
                    val: parsed.remainder.to_string(),
                    span: self.span,
                };
                self.stack.add_var(*buffer_id, line_buffer);
            }
        }

        if let Some(position) = block.signature.get_positional(1) {
            if let Some(position_id) = &position.var_id {
                let line_buffer = Value::Int {
                    val: pos as i64,
                    span: self.span,
                };
                self.stack.add_var(*position_id, line_buffer);
            }
        }

        let input = Value::nothing(self.span).into_pipeline_data();
        let res = eval_block(
            &self.engine_state,
            &mut self.stack,
            block,
            input,
            false,
            false,
        );

        if let Ok(values) = res {
            let values = values.into_value(self.span);
            convert_to_suggestions(values, line, pos, self.only_buffer_difference)
        } else {
            Vec::new()
        }
    }
}

fn convert_to_suggestions(
    value: Value,
    line: &str,
    pos: usize,
    only_buffer_difference: bool,
) -> Vec<Suggestion> {
    match value {
        Value::Record { .. } => {
            let text = match value
                .get_data_by_key("value")
                .and_then(|val| val.as_string().ok())
            {
                Some(val) => val,
                None => "No value key".to_string(),
            };

            let description = value
                .get_data_by_key("description")
                .and_then(|val| val.as_string().ok());

            let span = match value.get_data_by_key("span") {
                Some(span @ Value::Record { .. }) => {
                    let start = span
                        .get_data_by_key("start")
                        .and_then(|val| val.as_integer().ok());
                    let end = span
                        .get_data_by_key("end")
                        .and_then(|val| val.as_integer().ok());
                    match (start, end) {
                        (Some(start), Some(end)) => {
                            let start = start.min(end);
                            reedline::Span {
                                start: start as usize,
                                end: end as usize,
                            }
                        }
                        _ => reedline::Span {
                            start: if only_buffer_difference { pos } else { 0 },
                            end: if only_buffer_difference {
                                pos + line.len()
                            } else {
                                line.len()
                            },
                        },
                    }
                }
                _ => reedline::Span {
                    start: if only_buffer_difference { pos } else { 0 },
                    end: if only_buffer_difference {
                        pos + line.len()
                    } else {
                        line.len()
                    },
                },
            };

            let extra = match value.get_data_by_key("extra") {
                Some(Value::List { vals, .. }) => {
                    let extra: Vec<String> = vals
                        .into_iter()
                        .filter_map(|extra| match extra {
                            Value::String { val, .. } => Some(val),
                            _ => None,
                        })
                        .collect();

                    Some(extra)
                }
                _ => None,
            };

            vec![Suggestion {
                value: text,
                description,
                extra,
                span,
                append_whitespace: false,
            }]
        }
        Value::List { vals, .. } => vals
            .into_iter()
            .flat_map(|val| convert_to_suggestions(val, line, pos, only_buffer_difference))
            .collect(),
        _ => vec![Suggestion {
            value: format!("Not a record: {:?}", value),
            description: None,
            extra: None,
            span: reedline::Span {
                start: 0,
                end: line.len(),
            },
            append_whitespace: false,
        }],
    }
}
