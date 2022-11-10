use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

struct Arguments {
    pattern: Vec<u8>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]

pub struct BytesEndsWith;

impl Command for BytesEndsWith {
    fn name(&self) -> &str {
        "bytes ends-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes ends-with")
            .input_output_types(vec![(Type::Binary, Type::Bool)])
            .required("pattern", SyntaxShape::Binary, "the pattern to match")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, check if bytes at the given cell paths end with the pattern",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Check if bytes ends with a pattern"
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
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments {
            pattern,
            cell_paths,
        };
        operate(ends_with, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if binary ends with `0x[AA]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[AA]",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if binary ends with `0x[FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[FF AA AA]",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if binary ends with `0x[11]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[11]",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn ends_with(val: &Value, args: &Arguments, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => Value::Bool {
            val: val.ends_with(&args.pattern),
            span: *val_span,
        },
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

        test_examples(BytesEndsWith {})
    }
}
