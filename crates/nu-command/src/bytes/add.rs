use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

struct Arguments {
    added_data: Vec<u8>,
    index: Option<usize>,
    end: bool,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]

pub struct BytesAdd;

impl Command for BytesAdd {
    fn name(&self) -> &str {
        "bytes add"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes add")
            .input_output_types(vec![(Type::Binary, Type::Binary)])
            .vectorizes_over_list(true)
            .required("data", SyntaxShape::Binary, "the binary to add")
            .named(
                "index",
                SyntaxShape::Int,
                "index to insert binary data",
                Some('i'),
            )
            .switch("end", "add to the end of binary", Some('e'))
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, add bytes to the data at the given cell paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Add specified bytes to the input"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["append", "truncate", "padding"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let added_data: Vec<u8> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let index: Option<usize> = call.get_flag(engine_state, stack, "index")?;
        let end = call.has_flag("end");

        let arg = Arguments {
            added_data,
            index,
            end,
            cell_paths,
        };
        operate(add, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Add bytes `0x[AA]` to `0x[1F FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes add 0x[AA]",
                result: Some(Value::Binary {
                    val: vec![0xAA, 0x1F, 0xFF, 0xAA, 0xAA],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Add bytes `0x[AA BB]` to `0x[1F FF AA AA]` at index 1",
                example: "0x[1F FF AA AA] | bytes add 0x[AA BB] -i 1",
                result: Some(Value::Binary {
                    val: vec![0x1F, 0xAA, 0xBB, 0xFF, 0xAA, 0xAA],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Add bytes `0x[11]` to `0x[FF AA AA]` at the end",
                example: "0x[FF AA AA] | bytes add 0x[11] -e",
                result: Some(Value::Binary {
                    val: vec![0xFF, 0xAA, 0xAA, 0x11],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Add bytes `0x[11 22 33]` to `0x[FF AA AA]` at the end, at index 1(the index is start from end)",
                example: "0x[FF AA BB] | bytes add 0x[11 22 33] -e -i 1",
                result: Some(Value::Binary {
                    val: vec![0xFF, 0xAA, 0x11, 0x22, 0x33, 0xBB],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn add(val: &Value, args: &Arguments, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => add_impl(val, args, *val_span),
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

fn add_impl(input: &[u8], args: &Arguments, span: Span) -> Value {
    match args.index {
        None => {
            if args.end {
                let mut added_data = args.added_data.clone();
                let mut result = input.to_vec();
                result.append(&mut added_data);
                Value::Binary { val: result, span }
            } else {
                let mut result = args.added_data.clone();
                let mut input = input.to_vec();
                result.append(&mut input);
                Value::Binary { val: result, span }
            }
        }
        Some(mut indx) => {
            let inserted_index = if args.end {
                input.len().saturating_sub(indx)
            } else {
                if indx > input.len() {
                    indx = input.len()
                }
                indx
            };
            let mut result = vec![];
            let mut prev_data = input[..inserted_index].to_vec();
            result.append(&mut prev_data);
            let mut added_data = args.added_data.clone();
            result.append(&mut added_data);
            let mut after_data = input[inserted_index..].to_vec();
            result.append(&mut after_data);
            Value::Binary { val: result, span }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesAdd {})
    }
}
