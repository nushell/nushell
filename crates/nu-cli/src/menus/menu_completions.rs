use nu_engine::eval_block;
use nu_protocol::{
    BlockId, IntoPipelineData, Span, Value,
    debugger::WithoutDebug,
    engine::{EngineState, Stack},
};
use reedline::{Completer, Suggestion, menu_functions::parse_selection_char};
use std::sync::Arc;

const SELECTION_CHAR: char = '!';

pub struct NuMenuCompleter {
    block_id: BlockId,
    span: Span,
    stack: Stack,
    engine_state: Arc<EngineState>,
    only_buffer_difference: bool,
}

impl NuMenuCompleter {
    pub fn new(
        block_id: BlockId,
        span: Span,
        stack: Stack,
        engine_state: Arc<EngineState>,
        only_buffer_difference: bool,
    ) -> Self {
        Self {
            block_id,
            span,
            stack: stack.reset_out_dest().collect_value(),
            engine_state,
            only_buffer_difference,
        }
    }
}

impl Completer for NuMenuCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let parsed = parse_selection_char(line, SELECTION_CHAR);

        let block = self.engine_state.get_block(self.block_id);

        if let Some(buffer) = block.signature.get_positional(0)
            && let Some(buffer_id) = &buffer.var_id
        {
            let line_buffer = Value::string(parsed.remainder, self.span);
            self.stack.add_var(*buffer_id, line_buffer);
        }

        if let Some(position) = block.signature.get_positional(1)
            && let Some(position_id) = &position.var_id
        {
            let line_buffer = Value::int(pos as i64, self.span);
            self.stack.add_var(*position_id, line_buffer);
        }

        let input = Value::nothing(self.span).into_pipeline_data();

        let res = eval_block::<WithoutDebug>(&self.engine_state, &mut self.stack, block, input)
            .map(|p| p.body);

        if let Ok(values) = res.and_then(|data| data.into_value(self.span)) {
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
        Value::Record { val, .. } => {
            let text = val
                .get("value")
                .and_then(|val| val.coerce_string().ok())
                .unwrap_or_else(|| "No value key".to_string());

            let description = val
                .get("description")
                .and_then(|val| val.coerce_string().ok());

            let span = match val.get("span") {
                Some(Value::Record { val: span, .. }) => {
                    let start = span.get("start").and_then(|val| val.as_int().ok());
                    let end = span.get("end").and_then(|val| val.as_int().ok());
                    match (start, end) {
                        (Some(start), Some(end)) => {
                            let start = start.min(end);
                            reedline::Span {
                                start: start as usize,
                                end: end as usize,
                            }
                        }
                        _ => reedline::Span {
                            start: if only_buffer_difference {
                                pos - line.len()
                            } else {
                                0
                            },
                            end: if only_buffer_difference {
                                pos
                            } else {
                                line.len()
                            },
                        },
                    }
                }
                _ => reedline::Span {
                    start: if only_buffer_difference {
                        pos - line.len()
                    } else {
                        0
                    },
                    end: if only_buffer_difference {
                        pos
                    } else {
                        line.len()
                    },
                },
            };

            let extra = match val.get("extra") {
                Some(Value::List { vals, .. }) => {
                    let extra: Vec<String> = vals
                        .iter()
                        .filter_map(|extra| match extra {
                            Value::String { val, .. } => Some(val.clone()),
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
                ..Suggestion::default()
            }]
        }
        Value::List { vals, .. } => vals
            .into_iter()
            .flat_map(|val| convert_to_suggestions(val, line, pos, only_buffer_difference))
            .collect(),
        _ => vec![Suggestion {
            value: format!("Not a record: {value:?}"),
            span: reedline::Span {
                start: if only_buffer_difference {
                    pos - line.len()
                } else {
                    0
                },
                end: if only_buffer_difference {
                    pos
                } else {
                    line.len()
                },
            },
            ..Suggestion::default()
        }],
    }
}
