use nu_engine::eval_block;
use nu_protocol::{
    engine::{EngineState, Stack},
    IntoPipelineData, Span, Value,
};
use reedline::{Completer, Suggestion};

pub struct NuMenuCompleter {
    block_id: usize,
    span: Span,
    stack: Stack,
    engine_state: EngineState,
}

impl NuMenuCompleter {
    pub fn new(block_id: usize, span: Span, stack: Stack, engine_state: EngineState) -> Self {
        Self {
            block_id,
            span,
            stack,
            engine_state,
        }
    }
}

impl Completer for NuMenuCompleter {
    fn complete(&self, line: &str, _pos: usize) -> Vec<Suggestion> {
        let block = self.engine_state.get_block(self.block_id);
        let mut stack = self.stack.clone();

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                let line_buffer = Value::String {
                    val: line.to_string(),
                    span: self.span,
                };
                stack.add_var(*var_id, line_buffer);
            }
        }

        let input = Value::nothing(self.span).into_pipeline_data();
        let res = eval_block(&self.engine_state, &mut stack, block, input, false, false);

        if let Ok(values) = res {
            let values = values.into_value(self.span);
            convert_to_suggestions(values, line)
        } else {
            Vec::new()
        }
    }
}

fn convert_to_suggestions(value: Value, line: &str) -> Vec<Suggestion> {
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
                Some(Value::Record { .. }) => unimplemented!(),
                Some(_) | None => reedline::Span {
                    start: 0,
                    end: line.len(),
                },
            };

            vec![Suggestion {
                value: text,
                description,
                extra: None,
                span,
            }]
        }
        Value::List { vals, .. } => vals
            .into_iter()
            .flat_map(|val| convert_to_suggestions(val, line))
            .collect(),
        _ => vec![Suggestion {
            value: "Nothing found".to_string(),
            description: None,
            extra: None,
            span: reedline::Span {
                start: 0,
                end: line.len(),
            },
        }],
    }
}
