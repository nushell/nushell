use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

#[derive(Clone)]
pub struct BytesLen;

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
        operate(engine_state, stack, call, input)
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

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    if column_paths.is_empty() {
        input.map(move |v| action(&v, head), engine_state.ctrlc.clone())
    } else {
        input.map(
            move |mut v| {
                for path in &column_paths {
                    let r =
                        v.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                v
            },
            engine_state.ctrlc.clone(),
        )
    }
}

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::Binary { val, .. } => Value::Int {
            val: val.len() as i64,
            span: head,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with bytes.",
                    other.get_type()
                ),
                head,
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
