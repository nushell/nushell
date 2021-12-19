use std::convert::TryInto;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Keep;

impl Command for Keep {
    fn name(&self) -> &str {
        "keep"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("n", SyntaxShape::Int, "the number of elements to keep")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Keep the first n elements of the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Keep two elements",
                example: "echo [[editions]; [2015] [2018] [2021]] | keep 2",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["editions".to_owned()],
                            vals: vec![Value::test_int(2015)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["editions".to_owned()],
                            vals: vec![Value::test_int(2018)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Keep the first value",
                example: "echo [2 4 6 8] | keep",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2)],
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
        let n: Option<Value> = call.opt(engine_state, stack, 0)?;

        let n: usize = match n {
            Some(Value::Int { val, span }) => val.try_into().map_err(|err| {
                ShellError::UnsupportedInput(
                    format!("Could not convert {} to unsigned integer: {}", val, err),
                    span,
                )
            })?,
            Some(_) => {
                let span = call.head;
                return Err(ShellError::TypeMismatch("expected integer".into(), span));
            }
            None => 1,
        };

        let ctrlc = engine_state.ctrlc.clone();

        Ok(input.into_iter().take(n).into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Keep {})
    }
}
