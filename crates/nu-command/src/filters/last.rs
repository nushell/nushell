use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value};

use std::convert::TryInto;

#[derive(Clone)]
pub struct Last;

impl Command for Last {
    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the back, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the last number of rows."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let rows: Option<i64> = call.opt(engine_state, stack, 0)?;

        /*
                let vlength = length(input)?;
                dbg!(vlength);

                let end_rows_desired = if let Some(quantity) = rows {
                    quantity
                } else {
                    1
                };

                let beginning_rows_to_skip = if end_rows_desired < vlength {
                    vlength - end_rows_desired
                } else {
                    0
                };

                dbg!(beginning_rows_to_skip);
        */

        let beginning_rows_to_skip = 2;

        match input {
            PipelineData::Stream(stream) => {
                dbg!("Stream");
                Ok(stream
                    .skip(beginning_rows_to_skip.try_into().unwrap())
                    .into_pipeline_data())
            }
            PipelineData::Value(Value::List { vals, .. }) => {
                dbg!("Value");
                Ok(vals
                    .into_iter()
                    .skip(beginning_rows_to_skip.try_into().unwrap())
                    .into_pipeline_data())
            }
            _ => {
                dbg!("Fall to the bottom");
                Ok(PipelineData::Value(Value::Nothing { span: call.head }))
            }
        }

        // Ok(PipelineData::Value(Value::Nothing { span: call.head }))
    }
}

fn length(input: PipelineData) -> Result<i64, nu_protocol::ShellError> {
    match input {
        PipelineData::Value(Value::Nothing { .. }) => Ok(1),
        _ => Ok(input.into_iter().count() as i64),
    }
}
