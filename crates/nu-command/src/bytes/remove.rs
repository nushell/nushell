use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::{
    ast::{Call, CellPath},
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

struct Arguments {
    pattern: Vec<u8>,
    end: bool,
    column_paths: Option<Vec<CellPath>>,
    all: bool,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
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
            .required("pattern", SyntaxShape::Binary, "the pattern to find")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally remove bytes by column paths",
            )
            .switch("end", "remove from end of binary", Some('e'))
            .switch("all", "remove occurrences of finding binary", Some('a'))
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "remove bytes"
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
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let pattern_to_remove = call.req::<Spanned<Vec<u8>>>(engine_state, stack, 0)?;
        if pattern_to_remove.item.is_empty() {
            return Err(ShellError::UnsupportedInput(
                "the pattern to remove cannot be empty".to_string(),
                pattern_to_remove.span,
            ));
        }

        let pattern_to_remove: Vec<u8> = pattern_to_remove.item;
        let arg = Arguments {
            pattern: pattern_to_remove,
            end: call.has_flag("end"),
            column_paths,
            all: call.has_flag("all"),
        };

        operate(remove, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove contents",
                example: "0x[10 AA FF AA FF] | bytes remove 0x[10 AA]",
                result: Some(Value::Binary {
                    val: vec![0xFF, 0xAA, 0xFF],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Remove all occurrences of find binary",
                example: "0x[10 AA 10 BB 10] | bytes remove -a 0x[10]",
                result: Some(Value::Binary {
                    val: vec![0xAA, 0xBB],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Remove occurrences of find binary from end",
                example: "0x[10 AA 10 BB CC AA 10] | bytes remove -e 0x[10]",
                result: Some(Value::Binary {
                    val: vec![0x10, 0xAA, 0x10, 0xBB, 0xCC, 0xAA],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Remove all occurrences of find binary in table",
                example: "[[ColA ColB ColC]; [0x[11 12 13] 0x[14 15 16] 0x[17 18 19]]] | bytes remove 0x[11] ColA ColC",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string(), "ColC".to_string()],
                        vals: vec![
                            Value::Binary {
                                val: vec![0x12, 0x13],
                                span: Span::test_data(),
                            },
                            Value::Binary {
                                val: vec![0x14, 0x15, 0x16],
                                span: Span::test_data(),
                            },
                            Value::Binary {
                                val: vec![0x17, 0x18, 0x19],
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

fn remove(input: &[u8], arg: &Arguments, span: Span) -> Value {
    let mut result = vec![];
    let remove_all = arg.all;
    let input_len = input.len();
    let pattern_len = arg.pattern.len();

    // Note:
    // remove_all from start and end will generate the same result.
    // so we'll put `remove_all` relative logic into else clouse.
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
        // append the remaining thing to result, this can be happeneed when
        // we have something to remove and remove_all is False.
        let mut remain = input[..left as usize].iter().copied().rev().collect();
        result.append(&mut remain);
        result = result.into_iter().rev().collect();
        Value::Binary { val: result, span }
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
        // append the remaing thing to result, this can happened when
        // we have something to remove and remove_all is False.
        let mut remain = input[left..].to_vec();
        result.append(&mut remain);
        Value::Binary { val: result, span }
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
