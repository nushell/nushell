use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Every;

impl Command for Every {
    fn name(&self) -> &str {
        "every"
    }

    fn signature(&self) -> Signature {
        Signature::build("every")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "stride",
                SyntaxShape::Int,
                "how many rows to skip between (and including) each row returned",
            )
            .switch(
                "skip",
                "skip the rows that would be returned, instead of selecting them",
                Some('s'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Show (or skip) every n-th row, starting from the first one."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[1 2 3 4 5] | every 2",
                description: "Get every second row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[1 2 3 4 5] | every 2 --skip",
                description: "Skip every second row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(4)],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let stride = match call.req::<usize>(engine_state, stack, 0)? {
            0 => 1,
            stride => stride,
        };

        let skip = call.has_flag("skip");

        let metadata = input.metadata();

        Ok(input
            .into_iter()
            .enumerate()
            .filter_map(move |(i, value)| {
                if (i % stride != 0) == skip {
                    Some(value)
                } else {
                    None
                }
            })
            .into_pipeline_data(engine_state.ctrlc.clone())
            .set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Every {})
    }
}
