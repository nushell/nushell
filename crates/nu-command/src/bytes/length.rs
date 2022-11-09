use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct BytesLen;

impl Command for BytesLen {
    fn name(&self) -> &str {
        "bytes length"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes length")
            .input_output_types(vec![(Type::Binary, Type::Int)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, find the length of data at the given cell paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Output the length of any bytes in the pipeline"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["size", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let arg = CellPathOnlyArgs::from(cell_paths);
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

fn length(val: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => Value::Int {
            val: val.len() as i64,
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
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesLen {})
    }
}
