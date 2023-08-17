use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_cmd_base::util;
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Range, ShellError, Signature, Span, SpannedValue, SyntaxShape,
    Type,
};

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
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .required("range", SyntaxShape::Range, "the range to get bytes")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, get bytes from data at the given cell paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Get bytes defined by a range"
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

        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a subbytes `0x[10 01]` from the bytes `0x[33 44 55 10 01 13]`",
                example: " 0x[33 44 55 10 01 13] | bytes at 3..<4",
                result: Some(SpannedValue::Binary {
                    val: vec![0x10],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get a subbytes `0x[10 01 13]` from the bytes `0x[33 44 55 10 01 13]`",
                example: " 0x[33 44 55 10 01 13] | bytes at 3..6",
                result: Some(SpannedValue::Binary {
                    val: vec![0x10, 0x01, 0x13],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get the remaining characters from a starting index",
                example: " { data: 0x[33 44 55 10 01 13] } | bytes at 3.. data",
                result: Some(SpannedValue::test_record(
                    vec!["data"],
                    vec![SpannedValue::test_binary(vec![0x10, 0x01, 0x13])],
                )),
            },
            Example {
                description: "Get the characters from the beginning until ending index",
                example: " 0x[33 44 55 10 01 13] | bytes at ..<4",
                result: Some(SpannedValue::Binary {
                    val: vec![0x33, 0x44, 0x55, 0x10],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Or the characters from the beginning until ending index inside a table",
                example: r#" [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes at 1.. ColB ColC"#,
                result: Some(SpannedValue::List {
                    vals: vec![SpannedValue::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            SpannedValue::Binary {
                                val: vec![0x11, 0x12, 0x13],
                                span: Span::test_data(),
                            },
                            SpannedValue::Binary {
                                val: vec![0x15, 0x16],
                                span: Span::test_data(),
                            },
                            SpannedValue::Binary {
                                val: vec![0x18, 0x19],
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &SpannedValue, args: &Arguments, head: Span) -> SpannedValue {
    let range = &args.indexes;
    match input {
        SpannedValue::Binary { val, .. } => {
            use std::cmp::{self, Ordering};
            let len = val.len() as isize;

            let start = if range.0 < 0 { range.0 + len } else { range.0 };

            let end = if range.1 < 0 {
                cmp::max(range.1 + len, 0)
            } else {
                range.1
            };

            if start < len && end >= 0 {
                match start.cmp(&end) {
                    Ordering::Equal => SpannedValue::Binary {
                        val: vec![],
                        span: head,
                    },
                    Ordering::Greater => SpannedValue::Error {
                        error: Box::new(ShellError::TypeMismatch {
                            err_message: "End must be greater than or equal to Start".to_string(),
                            span: head,
                        }),
                        span: head,
                    },
                    Ordering::Less => SpannedValue::Binary {
                        val: {
                            if end == isize::max_value() {
                                val.iter().skip(start as usize).copied().collect()
                            } else {
                                val.iter()
                                    .skip(start as usize)
                                    .take((end - start) as usize)
                                    .copied()
                                    .collect()
                            }
                        },
                        span: head,
                    },
                }
            } else {
                SpannedValue::Binary {
                    val: vec![],
                    span: head,
                }
            }
        }

        SpannedValue::Error { .. } => input.clone(),

        other => SpannedValue::Error {
            error: Box::new(ShellError::UnsupportedInput(
                "Only binary values are supported".into(),
                format!("input type: {:?}", other.get_type()),
                head,
                // This line requires the Value::Error match above.
                other.span(),
            )),
            span: head,
        },
    }
}
