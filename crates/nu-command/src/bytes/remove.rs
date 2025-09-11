use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

struct Arguments {
    pattern: Vec<u8>,
    end: bool,
    cell_paths: Option<Vec<CellPath>>,
    all: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct BytesRemove;

impl Command for BytesRemove {
    fn name(&self) -> &str {
        "bytes remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes remove")
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .required("pattern", SyntaxShape::Binary, "The pattern to find.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, remove bytes from data at the given cell paths.",
            )
            .switch("end", "remove from end of binary", Some('e'))
            .switch("all", "remove occurrences of finding binary", Some('a'))
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Remove bytes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["search", "shift", "switch"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let pattern_to_remove = call.req::<Spanned<Vec<u8>>>(engine_state, stack, 0)?;
        if pattern_to_remove.item.is_empty() {
            return Err(ShellError::TypeMismatch {
                err_message: "the pattern to remove cannot be empty".to_string(),
                span: pattern_to_remove.span,
            });
        }

        let pattern_to_remove: Vec<u8> = pattern_to_remove.item;
        let arg = Arguments {
            pattern: pattern_to_remove,
            end: call.has_flag(engine_state, stack, "end")?,
            cell_paths,
            all: call.has_flag(engine_state, stack, "all")?,
        };

        operate(remove, arg, input, call.head, engine_state.signals()).map(|pipeline| {
            // image/png with some bytes removed is likely not a valid image/png anymore
            let metadata = pipeline.metadata().map(|m| m.with_content_type(None));
            pipeline.set_metadata(metadata)
        })
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Remove contents",
                example: "0x[10 AA FF AA FF] | bytes remove 0x[10 AA]",
                result: Some(Value::test_binary(vec![0xFF, 0xAA, 0xFF])),
            },
            Example {
                description: "Remove all occurrences of find binary in record field",
                example: "{ data: 0x[10 AA 10 BB 10] } | bytes remove --all 0x[10] data",
                result: Some(Value::test_record(record! {
                    "data" => Value::test_binary(vec![0xAA, 0xBB])
                })),
            },
            Example {
                description: "Remove occurrences of find binary from end",
                example: "0x[10 AA 10 BB CC AA 10] | bytes remove --end 0x[10]",
                result: Some(Value::test_binary(vec![0x10, 0xAA, 0x10, 0xBB, 0xCC, 0xAA])),
            },
            Example {
                description: "Remove find binary from end not found",
                example: "0x[10 AA 10 BB CC AA 10] | bytes remove --end 0x[11]",
                result: Some(Value::test_binary(vec![
                    0x10, 0xAA, 0x10, 0xBB, 0xCC, 0xAA, 0x10,
                ])),
            },
            Example {
                description: "Remove all occurrences of find binary in table",
                example: "[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes remove 0x[11] ColA ColC",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_binary ( vec![0x12, 0x13],),
                    "ColB" => Value::test_binary ( vec![0x14, 0x15, 0x16],),
                    "ColC" => Value::test_binary ( vec![0x17, 0x18, 0x19],),
                })])),
            },
        ]
    }
}

fn remove(val: &Value, args: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => remove_impl(val, args, val_span),
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

fn remove_impl(input: &[u8], arg: &Arguments, span: Span) -> Value {
    let mut result = vec![];
    let remove_all = arg.all;
    let input_len = input.len();
    let pattern_len = arg.pattern.len();

    // Note:
    // remove_all from start and end will generate the same result.
    // so we'll put `remove_all` relative logic into else clause.
    if arg.end && !remove_all {
        let (mut left, mut right) = (
            input.len() as isize - arg.pattern.len() as isize,
            input.len() as isize,
        );
        while left >= 0 && input[left as usize..right as usize] != arg.pattern {
            result.push(input[right as usize - 1]);
            left -= 1;
            right -= 1;
        }
        // append the remaining thing to result, this can be happening when
        // we have something to remove and remove_all is False.
        // check if the left is positive, if it is not, we don't need to append anything.
        if left > 0 {
            let mut remain = input[..left as usize].iter().copied().rev().collect();
            result.append(&mut remain);
        }
        result = result.into_iter().rev().collect();
        Value::binary(result, span)
    } else {
        let (mut left, mut right) = (0, arg.pattern.len());
        while right <= input_len {
            if input[left..right] == arg.pattern {
                left += pattern_len;
                right += pattern_len;
                if !remove_all {
                    break;
                }
            } else {
                result.push(input[left]);
                left += 1;
                right += 1;
            }
        }
        // append the remaining thing to result, this can happened when
        // we have something to remove and remove_all is False.
        let mut remain = input[left..].to_vec();
        result.append(&mut remain);
        Value::binary(result, span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesRemove {})
    }
}
