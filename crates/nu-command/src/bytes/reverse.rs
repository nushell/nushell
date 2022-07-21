use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

struct Arguments {
    column_paths: Option<Vec<CellPath>>,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]

pub struct BytesReverse;

impl Command for BytesReverse {
    fn name(&self) -> &str {
        "bytes reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes reverse")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally matches prefix of text by column paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Reverse every bytes in the pipeline"
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
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments { column_paths };
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

fn reverse(input: &[u8], _args: &Arguments, span: Span) -> Value {
    let mut reversed_input = input.to_vec();
    reversed_input.reverse();
    Value::Binary {
        val: reversed_input,
        span,
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
