use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct First;

impl Command for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first").optional(
            "rows",
            SyntaxShape::Int,
            "starting from the front, the number of rows to return",
        )
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        first_helper(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first item of a list/table",
                example: "[1 2 3] | first",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Return the first 2 items of a list/table",
                example: "[1 2 3] | first 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

fn first_helper(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let rows: Option<i64> = call.opt(engine_state, stack, 0)?;
    let rows_desired: usize = match rows {
        Some(x) => x as usize,
        None => 1,
    };

    if rows_desired == 1 {
        let mut input_peek = input.into_iter().peekable();
        match input_peek.next() {
            Some(val) => Ok(val.into_pipeline_data()),
            None => Err(ShellError::AccessBeyondEndOfStream(head)),
        }
    } else {
        Ok(Value::List {
            vals: input.into_iter().take(rows_desired).collect(),
            span: head,
        }
        .into_pipeline_data())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(First {})
    }
}
