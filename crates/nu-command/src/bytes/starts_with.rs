use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

struct Arguments {
    pattern: Vec<u8>,
    column_paths: Option<Vec<CellPath>>,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]

pub struct BytesStartsWith;

impl Command for BytesStartsWith {
    fn name(&self) -> &str {
        "bytes starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes starts-with")
            .required("pattern", SyntaxShape::Binary, "the pattern to match")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally matches prefix of text by column paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Check if bytes starts with a pattern"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Vec<u8> = call.req(engine_state, stack, 0)?;
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments {
            pattern,
            column_paths,
        };
        operate(
            starts_with,
            arg,
            input,
            call.head,
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if binary starts with `0x[1F FF AA]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F FF AA]",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F]",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[11]",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn starts_with(input: &[u8], Arguments { pattern, .. }: &Arguments, span: Span) -> Value {
    Value::Bool {
        val: input.starts_with(pattern),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesStartsWith {})
    }
}
