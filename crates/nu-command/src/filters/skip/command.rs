use std::convert::TryInto;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Skip;

impl Command for Skip {
    fn name(&self) -> &str {
        "skip"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("n", SyntaxShape::Int, "the number of elements to skip")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Skip the first n elements of the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Skip two elements",
                example: "echo [[editions]; [2015] [2018] [2021]] | skip 2",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["editions".to_owned()],
                        vals: vec![Value::from(2021)],
                        span: Span::unknown(),
                    }],
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Skip the first value",
                example: "echo [2 4 6 8] | skip",
                result: Some(Value::List {
                    vals: vec![Value::from(4), Value::from(6), Value::from(8)],
                    span: Span::unknown(),
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
        let n: Option<Value> = call.opt(engine_state, stack, 0)?;
        let span = call.head;

        let n: usize = match n {
            Some(Value::Int { val, span }) => val.try_into().map_err(|err| {
                ShellError::UnsupportedInput(
                    format!("Could not convert {} to unsigned integer: {}", val, err),
                    span,
                )
            })?,
            Some(_) => return Err(ShellError::TypeMismatch("expected integer".into(), span)),
            None => 1,
        };

        let ctrlc = engine_state.ctrlc.clone();

        Ok(input.into_iter().skip(n).into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Skip {})
    }
}
