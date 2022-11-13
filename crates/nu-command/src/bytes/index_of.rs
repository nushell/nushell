use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

struct Arguments {
    pattern: Vec<u8>,
    end: bool,
    all: bool,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct BytesIndexOf;

impl Command for BytesIndexOf {
    fn name(&self) -> &str {
        "bytes index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes index-of")
            .input_output_types(vec![
                (Type::Binary, Type::Int),
                (Type::Binary, Type::List(Box::new(Type::Int))),
            ])
            .required(
                "pattern",
                SyntaxShape::Binary,
                "the pattern to find index of",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, find the indexes at the given cell paths",
            )
            .switch("all", "returns all matched index", Some('a'))
            .switch("end", "search from the end of the binary", Some('e'))
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Returns start index of first occurrence of pattern in bytes, or -1 if no match"
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
            end: call.has_flag("end"),
            all: call.has_flag("all"),
            cell_paths,
        };
        operate(index_of, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns index of pattern in bytes",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of 0x[44 55]",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Returns index of pattern, search from end",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of -e 0x[44 55]",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Returns all matched index",
                example: " 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a 0x[33 44]",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(5), Value::test_int(7)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Returns all matched index, searching from end",
                example: " 0x[33 44 55 10 01 33 44 33 44] | bytes index-of -a -e 0x[33 44]",
                result: Some(Value::List {
                    vals: vec![Value::test_int(7), Value::test_int(5), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Returns index of pattern for specific column",
                example: r#" [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes index-of 0x[11] ColA ColC"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::test_int(0),
                            Value::Binary {
                                val: vec![0x14, 0x15, 0x16],
                                span: Span::test_data(),
                            },
                            Value::test_int(-1),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn index_of(val: &Value, args: &Arguments, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => index_of_impl(val, args, *val_span),
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

fn index_of_impl(input: &[u8], arg: &Arguments, span: Span) -> Value {
    if arg.all {
        search_all_index(input, &arg.pattern, arg.end, span)
    } else {
        let mut iter = input.windows(arg.pattern.len());

        if arg.end {
            Value::Int {
                val: iter
                    .rev()
                    .position(|sub_bytes| sub_bytes == arg.pattern)
                    .map(|x| (input.len() - arg.pattern.len() - x) as i64)
                    .unwrap_or(-1),
                span,
            }
        } else {
            Value::Int {
                val: iter
                    .position(|sub_bytes| sub_bytes == arg.pattern)
                    .map(|x| x as i64)
                    .unwrap_or(-1),
                span,
            }
        }
    }
}

fn search_all_index(input: &[u8], pattern: &[u8], from_end: bool, span: Span) -> Value {
    let mut result = vec![];
    if from_end {
        let (mut left, mut right) = (
            input.len() as isize - pattern.len() as isize,
            input.len() as isize,
        );
        while left >= 0 {
            if &input[left as usize..right as usize] == pattern {
                result.push(Value::Int {
                    val: left as i64,
                    span,
                });
                left -= pattern.len() as isize;
                right -= pattern.len() as isize;
            } else {
                left -= 1;
                right -= 1;
            }
        }
        Value::List { vals: result, span }
    } else {
        // doing find stuff.
        let (mut left, mut right) = (0, pattern.len());
        let input_len = input.len();
        let pattern_len = pattern.len();
        while right <= input_len {
            if &input[left..right] == pattern {
                result.push(Value::Int {
                    val: left as i64,
                    span,
                });
                left += pattern_len;
                right += pattern_len;
            } else {
                left += 1;
                right += 1;
            }
        }

        Value::List { vals: result, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesIndexOf {})
    }
}
