use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

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
                (Type::Binary, Type::Any),
                // FIXME: this shouldn't be needed, cell paths should work with the two
                // above
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required(
                "pattern",
                SyntaxShape::Binary,
                "The pattern to find index of.",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, find the indexes at the given cell paths.",
            )
            .switch("all", "returns all matched index", Some('a'))
            .switch("end", "search from the end of the binary", Some('e'))
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Returns start index of first occurrence of pattern in bytes, or -1 if no match."
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
            end: call.has_flag(engine_state, stack, "end")?,
            all: call.has_flag(engine_state, stack, "all")?,
            cell_paths,
        };
        operate(index_of, arg, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns index of pattern in bytes",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of 0x[44 55]",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Returns index of pattern, search from end",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of --end 0x[44 55]",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Returns all matched index",
                example: " 0x[33 44 55 10 01 33 44 33 44] | bytes index-of --all 0x[33 44]",
                result: Some(Value::test_list(vec![
                    Value::test_int(0),
                    Value::test_int(5),
                    Value::test_int(7),
                ])),
            },
            Example {
                description: "Returns all matched index, searching from end",
                example: " 0x[33 44 55 10 01 33 44 33 44] | bytes index-of --all --end 0x[33 44]",
                result: Some(Value::test_list(vec![
                    Value::test_int(7),
                    Value::test_int(5),
                    Value::test_int(0),
                ])),
            },
            Example {
                description: "Returns index of pattern for specific column",
                example: r#" [[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes index-of 0x[11] ColA ColC"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_int(0),
                    "ColB" => Value::binary(vec![0x14, 0x15, 0x16], Span::test_data()),
                    "ColC" => Value::test_int(-1),
                })])),
            },
        ]
    }
}

fn index_of(val: &Value, args: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => index_of_impl(val, args, val_span),
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

fn index_of_impl(input: &[u8], arg: &Arguments, span: Span) -> Value {
    if arg.all {
        search_all_index(input, &arg.pattern, arg.end, span)
    } else {
        let mut iter = input.windows(arg.pattern.len());

        if arg.end {
            Value::int(
                iter.rev()
                    .position(|sub_bytes| sub_bytes == arg.pattern)
                    .map(|x| (input.len() - arg.pattern.len() - x) as i64)
                    .unwrap_or(-1),
                span,
            )
        } else {
            Value::int(
                iter.position(|sub_bytes| sub_bytes == arg.pattern)
                    .map(|x| x as i64)
                    .unwrap_or(-1),
                span,
            )
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
                result.push(Value::int(left as i64, span));
                left -= pattern.len() as isize;
                right -= pattern.len() as isize;
            } else {
                left -= 1;
                right -= 1;
            }
        }
        Value::list(result, span)
    } else {
        // doing find stuff.
        let (mut left, mut right) = (0, pattern.len());
        let input_len = input.len();
        let pattern_len = pattern.len();
        while right <= input_len {
            if &input[left..right] == pattern {
                result.push(Value::int(left as i64, span));
                left += pattern_len;
                right += pattern_len;
            } else {
                left += 1;
                right += 1;
            }
        }

        Value::list(result, span)
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
