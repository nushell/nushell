use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

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
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::Binary)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("data", SyntaxShape::Binary, "The binary to add.")
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
                "For a data structure input, add bytes to the data at the given cell paths.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Add specified bytes to the input."
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
        let end = call.has_flag(engine_state, stack, "end")?;

        let arg = Arguments {
            added_data,
            index,
            end,
            cell_paths,
        };
        operate(add, arg, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Add bytes `0x[AA]` to `0x[1F FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes add 0x[AA]",
                result: Some(Value::binary(
                    vec![0xAA, 0x1F, 0xFF, 0xAA, 0xAA],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Add bytes `0x[AA BB]` to `0x[1F FF AA AA]` at index 1",
                example: "0x[1F FF AA AA] | bytes add 0x[AA BB] --index 1",
                result: Some(Value::binary(
                    vec![0x1F, 0xAA, 0xBB, 0xFF, 0xAA, 0xAA],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Add bytes `0x[11]` to `0x[FF AA AA]` at the end",
                example: "0x[FF AA AA] | bytes add 0x[11] --end",
                result: Some(Value::binary(
                    vec![0xFF, 0xAA, 0xAA, 0x11],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Add bytes `0x[11 22 33]` to `0x[FF AA AA]` at the end, at index 1(the index is start from end)",
                example: "0x[FF AA BB] | bytes add 0x[11 22 33] --end --index 1",
                result: Some(Value::binary(
                    vec![0xFF, 0xAA, 0x11, 0x22, 0x33, 0xBB],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn add(val: &Value, args: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => add_impl(val, args, val_span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => val.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

fn add_impl(input: &[u8], args: &Arguments, span: Span) -> Value {
    match args.index {
        None => {
            if args.end {
                let mut added_data = args.added_data.clone();
                let mut result = input.to_vec();
                result.append(&mut added_data);
                Value::binary(result, span)
            } else {
                let mut result = args.added_data.clone();
                let mut input = input.to_vec();
                result.append(&mut input);
                Value::binary(result, span)
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
            Value::binary(result, span)
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
