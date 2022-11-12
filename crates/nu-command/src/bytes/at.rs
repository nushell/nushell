use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct BytesAt;

struct Arguments {
    start: isize,
    end: isize,
    arg_span: Span,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

/// ensure given `range` is valid, and returns [start, end, val_span] pair.
fn parse_range(range: Value, head: Span) -> Result<(isize, isize, Span), ShellError> {
    let (start, end, span) = match range {
        Value::List { mut vals, span } => {
            if vals.len() != 2 {
                return Err(ShellError::UnsupportedInput(
                    "More than two indices given".to_string(),
                    span,
                ));
            } else {
                let end = vals.pop().expect("Already check has size 2");
                let end = match end {
                    Value::Int { val, .. } => val.to_string(),
                    Value::String { val, .. } => val,
                    other => {
                        return Err(ShellError::UnsupportedInput(
                            "could not perform subbytes. Expecting a string or int".to_string(),
                            other.span().unwrap_or(head),
                        ))
                    }
                };
                let start = vals.pop().expect("Already check has size 1");
                let start = match start {
                    Value::Int { val, .. } => val.to_string(),
                    Value::String { val, .. } => val,
                    other => {
                        return Err(ShellError::UnsupportedInput(
                            "could not perform subbytes. Expecting a string or int".to_string(),
                            other.span().unwrap_or(head),
                        ))
                    }
                };
                (start, end, span)
            }
        }
        Value::String { val, span } => {
            let splitted_result = val.split_once(',');
            match splitted_result {
                Some((start, end)) => (start.to_string(), end.to_string(), span),
                None => {
                    return Err(ShellError::UnsupportedInput(
                        "could not perform subbytes".to_string(),
                        span,
                    ))
                }
            }
        }
        other => {
            return Err(ShellError::UnsupportedInput(
                "could not perform subbytes".to_string(),
                other.span().unwrap_or(head),
            ))
        }
    };

    let start: isize = if start.is_empty() || start == "_" {
        0
    } else {
        match start.trim().parse() {
            Ok(s) => s,
            Err(_) => {
                return Err(ShellError::UnsupportedInput(
                    "could not perform subbytes".to_string(),
                    span,
                ))
            }
        }
    };
    let end: isize = if end.is_empty() || end == "_" {
        isize::max_value()
    } else {
        match end.trim().parse() {
            Ok(s) => s,
            Err(_) => {
                return Err(ShellError::UnsupportedInput(
                    "could not perform subbytes".to_string(),
                    span,
                ))
            }
        }
    };
    Ok((start, end, span))
}

impl Command for BytesAt {
    fn name(&self) -> &str {
        "bytes at"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes at")
            .input_output_types(vec![(Type::Binary, Type::Binary)])
            .vectorizes_over_list(true)
            .required("range", SyntaxShape::Any, "the indexes to get bytes")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, get bytes from data at the given cell paths",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Get bytes defined by a range. Note that the start is included but the end is excluded, and that the first byte is index 0."
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
        let range: Value = call.req(engine_state, stack, 0)?;
        let (start, end, arg_span) = parse_range(range, call.head)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments {
            start,
            end,
            arg_span,
            cell_paths,
        };
        operate(at, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a subbytes `0x[10 01]` from the bytes `0x[33 44 55 10 01 13]`",
                example: " 0x[33 44 55 10 01 13] | bytes at [3 4]",
                result: Some(Value::Binary {
                    val: vec![0x10],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Alternatively, you can use the form",
                example: " 0x[33 44 55 10 01 13] | bytes at '3,4'",
                result: Some(Value::Binary {
                    val: vec![0x10],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Drop the last `n` characters from the string",
                example: " 0x[33 44 55 10 01 13] | bytes at ',-3'",
                result: Some(Value::Binary {
                    val: vec![0x33, 0x44, 0x55],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get the remaining characters from a starting index",
                example: " 0x[33 44 55 10 01 13] | bytes at '3,'",
                result: Some(Value::Binary {
                    val: vec![0x10, 0x01, 0x13],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Get the characters from the beginning until ending index",
                example: " 0x[33 44 55 10 01 13] | bytes at ',4'",
                result: Some(Value::Binary {
                    val: vec![0x33, 0x44, 0x55, 0x10],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Or the characters from the beginning until ending index inside a table",
                example: r#" [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes at "1," ColB ColC"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::Binary {
                                val: vec![0x11, 0x12, 0x13],
                                span: Span::test_data(),
                            },
                            Value::Binary {
                                val: vec![0x15, 0x16],
                                span: Span::test_data(),
                            },
                            Value::Binary {
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

fn at(val: &Value, args: &Arguments, span: Span) -> Value {
    match val {
        Value::Binary {
            val,
            span: val_span,
        } => at_impl(val, args, *val_span),
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

fn at_impl(input: &[u8], arg: &Arguments, span: Span) -> Value {
    let len: isize = input.len() as isize;

    let start: isize = if arg.start < 0 {
        arg.start + len
    } else {
        arg.start
    };
    let end: isize = if arg.end < 0 {
        std::cmp::max(len + arg.end, 0)
    } else {
        arg.end
    };

    if start < len && end >= 0 {
        match start.cmp(&end) {
            Ordering::Equal => Value::Binary { val: vec![], span },
            Ordering::Greater => Value::Error {
                error: ShellError::UnsupportedInput(
                    "End must be greater than or equal to Start".to_string(),
                    arg.arg_span,
                ),
            },
            Ordering::Less => Value::Binary {
                val: {
                    let input_iter = input.iter().copied().skip(start as usize);
                    if end == isize::max_value() {
                        input_iter.collect()
                    } else {
                        input_iter.take((end - start) as usize).collect()
                    }
                },
                span,
            },
        }
    } else {
        Value::Binary { val: vec![], span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesAt {})
    }
}
