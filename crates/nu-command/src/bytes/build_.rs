use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct BytesBuild;

impl Command for BytesBuild {
    fn name(&self) -> &str {
        "bytes build"
    }

    fn usage(&self) -> &str {
        "Create bytes from the arguments."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["concatenate", "join"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("bytes build")
            .input_output_types(vec![(Type::Nothing, Type::Binary)])
            .rest("rest", SyntaxShape::Any, "list of bytes")
            .category(Category::Bytes)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "bytes build 0x[01 02] 0x[03] 0x[04]",
            description: "Builds binary data from 0x[01 02], 0x[03], 0x[04]",
            result: Some(Value::Binary {
                val: vec![0x01, 0x02, 0x03, 0x04],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let mut output = vec![];
        for expr in call.positional_iter() {
            let val = eval_expression(engine_state, stack, expr)?;
            match val {
                Value::Binary { mut val, .. } => output.append(&mut val),
                other => {
                    return Err(ShellError::UnsupportedInput(
                        "only support expression which yields to binary data".to_string(),
                        other.span().unwrap_or(call.head),
                    ))
                }
            }
        }

        Ok(Value::Binary {
            val: output,
            span: call.head,
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

        test_examples(BytesBuild {})
    }
}
