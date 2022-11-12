use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]

pub struct BytesReverse;

impl Command for BytesReverse {
    fn name(&self) -> &str {
        "bytes reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes reverse")
            .input_output_types(vec![(Type::Binary, Type::Binary)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, reverse data at the given cell paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Reverse the bytes in the pipeline"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "inverse", "flip"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let arg = CellPathOnlyArgs::from(cell_paths);
        operate(reverse, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Reverse bytes `0x[1F FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes reverse",
                result: Some(Value::Binary {
                    val: vec![0xAA, 0xAA, 0xFF, 0x1F],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Reverse bytes `0x[FF AA AA]`",
                example: "0x[FF AA AA] | bytes reverse",
                result: Some(Value::Binary {
                    val: vec![0xAA, 0xAA, 0xFF],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn reverse(val: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => {
            let mut reversed_input = val.to_vec();
            reversed_input.reverse();
            Value::Binary {
                val: reversed_input,
                span: *val_span,
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with bytes.",
                    other.get_type()
                ),
                span,
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesReverse {})
    }
}
