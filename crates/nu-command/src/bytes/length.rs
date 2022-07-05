use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

#[derive(Clone)]
pub struct BytesLen;

struct Arguments {
    column_paths: Option<Vec<CellPath>>,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

impl Command for BytesLen {
    fn name(&self) -> &str {
        "bytes length"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes length")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally find length of binary by column paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Output the length of any bytes in the pipeline"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["len", "size", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments { column_paths };
        operate(length, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the lengths of multiple strings",
                example: "0x[1F FF AA AB] | bytes length",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "Return the lengths of multiple strings",
                example: "[0x[1F FF AA AB] 0x[1F]] | bytes length",
                result: Some(Value::List {
                    vals: vec![Value::test_int(4), Value::test_int(1)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn length(input: &[u8], _arg: &Arguments, span: Span) -> Value {
    Value::Int {
        val: input.len() as i64,
        span,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesLen {})
    }
}
