use itertools::Itertools;
use nu_cmd_base::{
    input_handler::{operate, CmdArgument},
    util,
};
use nu_engine::command_prelude::*;
use nu_protocol::{Range, Reader};
use std::io::{Bytes, Read, Write};

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
        let indexes = match util::process_range(&range) {
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
            handle_byte_stream(&args, stream, call, metadata, engine_state)
        } else {
            operate(action, args, input, call.head, engine_state.signals())
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
                example: "0x[33 44 55 10 01 13 10] | bytes at 3..6",
                result: Some(Value::test_binary(vec![0x10, 0x01, 0x13, 0x10])),
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

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    let range = &args.indexes;
    match input {
        Value::Binary { val, .. } => read_bytes(val, range, head),
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

fn handle_byte_stream(
    args: &Arguments,
    stream: ByteStream,
    call: &Call,
    metadata: Option<nu_protocol::PipelineMetadata>,
    engine_state: &EngineState,
) -> Result<PipelineData, ShellError> {
    let idxs = args.indexes;
    match stream.reader() {
        Some(reader) => {
            let iter = reader.bytes();

            if idxs.0 < 0 || idxs.1 < 0 {
                match iter.try_len() {
                    Ok(_) => {
                        let vec = iter.filter_map(Result::ok).collect::<Vec<u8>>();
                        Ok(read_bytes(&vec, &idxs, call.head).into_pipeline_data_with_metadata(metadata))
                    }
                    _ => Err(ShellError::IncorrectValue {
                        msg:
                            "Negative range values cannot be used with streams that don't specify a length"
                                .into(),
                        val_span: call.head,
                        call_span: call.arguments_span(),
                    }),
                }
            } else {
                Ok(read_stream(iter, idxs, call, engine_state, metadata))
            }
        }
        None => Ok(PipelineData::empty()),
    }
}

fn read_bytes(val: &[u8], range: &Subbytes, head: Span) -> Value {
    let len = val.len() as isize;
    let start = if range.0 < 0 { range.0 + len } else { range.0 };
    let end = if range.1 < 0 { range.1 + len } else { range.1 };

    if start > end {
        Value::binary(vec![], head)
    } else {
        let val_iter = val.iter().skip(start as usize);
        Value::binary(
            if end == isize::MAX {
                val_iter.copied().collect::<Vec<u8>>()
            } else {
                val_iter.take((end - start + 1) as usize).copied().collect()
            },
            head,
        )
    }
}

fn read_stream(
    iter: Bytes<Reader>,
    range: Subbytes,
    call: &Call,
    engine_state: &EngineState,
    metadata: Option<nu_protocol::PipelineMetadata>,
) -> PipelineData {
    let start = range.0 as usize;
    let end = (range.1 - range.0) as usize;
    let mut iter = iter.skip(start).take(end);

    let stream = ByteStream::from_fn(
        call.head,
        engine_state.signals().clone(),
        ByteStreamType::Binary,
        move |buf| match iter.next() {
            Some(Ok(n)) if n > 0 => match buf.write(&[n]) {
                Ok(_) => Ok(true),
                Err(err) => Err(err.into()),
            },
            _ => Ok(false),
        },
    );

    PipelineData::ByteStream(stream, metadata)
}
