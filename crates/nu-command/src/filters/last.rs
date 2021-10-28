use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};
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

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "[1,2,3] | last 2",
            description: "Get the last 2 items",
            result: Some(Value::List {
                vals: vec![Value::test_int(2), Value::test_int(3)],
                span: Span::unknown(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let rows: Option<i64> = call.opt(engine_state, stack, 0)?;
        let v: Vec<_> = input.into_iter().collect();
        let vlen: i64 = v.len() as i64;
        let beginning_rows_to_skip = rows_to_skip(vlen, rows);

        let iter = v
            .into_iter()
            .skip(beginning_rows_to_skip.try_into().unwrap());

        Ok(iter.into_pipeline_data(engine_state.ctrlc.clone()))
    }
}

fn rows_to_skip(count: i64, rows: Option<i64>) -> i64 {
    let end_rows_desired = if let Some(quantity) = rows {
        quantity
    } else {
        1
    };

    if end_rows_desired < count {
        count - end_rows_desired
    } else {
        0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Last {})
    }
}
