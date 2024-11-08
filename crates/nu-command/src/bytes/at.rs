use nu_cmd_base::{
    input_handler::{operate, CmdArgument},
    util,
};
use nu_engine::command_prelude::*;
use nu_protocol::Range;

#[derive(Clone)]
pub struct BytesAt;

struct Arguments {
    indexes: Subbytes,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl From<(isize, isize)> for Subbytes {
    fn from(input: (isize, isize)) -> Self {
        Self(input.0, input.1)
    }
}

#[derive(Clone, Copy)]
struct Subbytes(isize, isize);

impl Command for BytesAt {
    fn name(&self) -> &str {
        "bytes at"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes at")
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
            .required("range", SyntaxShape::Range, "The range to get bytes.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, get bytes from data at the given cell paths.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Get bytes defined by a range."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["slice"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let range: Range = call.req(engine_state, stack, 0)?;
        let indexes: Subbytes = match util::process_range(&range) {
            Ok(idxs) => idxs.into(),
            Err(processing_error) => {
                return Err(processing_error("could not perform subbytes", call.head));
            }
        };

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            indexes,
            cell_paths,
        };

        if let PipelineData::ByteStream(stream, metadata) = input {
            let stream = stream.slice(
                call.head,
                call.arguments_span(),
                args.indexes.0,
                args.indexes.1,
            )?;

            Ok(PipelineData::ByteStream(stream, metadata))
        } else {
            operate(map_value, args, input, call.head, engine_state.signals())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Extract bytes starting from a specific index",
                example: "{ data: 0x[33 44 55 10 01 13 10] } | bytes at 3.. data",
                result: Some(Value::test_record(record! {
                    "data" => Value::test_binary(vec![0x10, 0x01, 0x13, 0x10]),
                })),
            },
            Example {
                description: "Slice out `0x[10 01 13]` from `0x[33 44 55 10 01 13]`",
                example: "0x[33 44 55 10 01 13] | bytes at 3..6",
                result: Some(Value::test_binary(vec![0x10, 0x01, 0x13])),
            },
            Example {
                description: "Extract bytes from the start up to a specific index",
                example: "0x[33 44 55 10 01 13 10] | bytes at ..4",
                result: Some(Value::test_binary(vec![0x33, 0x44, 0x55, 0x10, 0x01])),
            },
            Example {
                description: "Extract byte `0x[10]` using an exclusive end index",
                example: "0x[33 44 55 10 01 13 10] | bytes at 3..<4",
                result: Some(Value::test_binary(vec![0x10])),
            },
            Example {
                description: "Extract bytes up to a negative index (inclusive)",
                example: "0x[33 44 55 10 01 13 10] | bytes at ..-4",
                result: Some(Value::test_binary(vec![0x33, 0x44, 0x55, 0x10])),
            },
            Example {
                description: "Slice bytes across multiple table columns",
                example: r#"[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes at 1.. ColB ColC"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_binary(vec![0x11, 0x12, 0x13]),
                    "ColB" => Value::test_binary(vec![0x15, 0x16]),
                    "ColC" => Value::test_binary(vec![0x18, 0x19]),
                })])),
            },
            Example {
                description: "Extract the last three bytes using a negative start index",
                example: "0x[33 44 55 10 01 13 10] | bytes at (-3)..",
                result: Some(Value::test_binary(vec![0x01, 0x13, 0x10])),
            },
        ]
    }
}

fn map_value(input: &Value, args: &Arguments, head: Span) -> Value {
    let range = &args.indexes;
    match input {
        Value::Binary { val, .. } => {
            let (start, end) = resolve_relative_range(range, &val.len());
            let iter = val.iter().copied();

            let bytes: Vec<u8> = if start > end {
                vec![]
            } else if end == usize::MAX {
                iter.skip(start).collect()
            } else {
                iter.skip(start).take(end - start + 1).collect()
            };

            Value::binary(bytes, head)
        }
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::UnsupportedInput {
                msg: "Only binary values are supported".into(),
                input: format!("input type: {:?}", other.get_type()),
                msg_span: head,
                input_span: other.span(),
            },
            head,
        ),
    }
}

fn resolve_relative_range(range: &Subbytes, len: &usize) -> (usize, usize) {
    let start = match range.0 {
        start if start < 0 => len.checked_sub(start.abs() as usize).unwrap_or(0),
        start => start as usize,
    };

    let end = match range.1 {
        end if end < 0 => len.checked_sub(end.abs() as usize).unwrap_or(0),
        end => end as usize,
    };

    (start, end)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(BytesAt {})
    }
}
