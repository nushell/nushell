use nu_engine::command_prelude::*;
use nu_protocol::{PipelineIterator, Range};
use std::collections::VecDeque;
use std::ops::Bound;

#[derive(Clone)]
pub struct DropNth;

impl Command for DropNth {
    fn name(&self) -> &str {
        "drop nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop nth")
            .input_output_types(vec![
                (Type::Range, Type::list(Type::Number)),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::Any,
                "The row numbers or ranges to drop.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Drop the selected rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove", "index"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[sam,sarah,2,3,4,5] | drop nth 0 1 2",
                description: "Drop the first, second, and third row",
                result: Some(Value::list(
                    vec![Value::test_int(3), Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 0 1 2",
                description: "Drop the first, second, and third row",
                result: Some(Value::list(
                    vec![Value::test_int(3), Value::test_int(4), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 0 2 4",
                description: "Drop rows 0 2 4",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 2 0 4",
                description: "Drop rows 2 0 4",
                result: Some(Value::list(
                    vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Drop range rows from second to fourth",
                example: "[first second third fourth fifth] | drop nth (1..3)",
                result: Some(Value::list(
                    vec![Value::test_string("first"), Value::test_string("fifth")],
                    Span::test_data(),
                )),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 1..",
                description: "Drop all rows except first row",
                result: Some(Value::list(vec![Value::test_int(0)], Span::test_data())),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 3..",
                description: "Drop rows 3,4,5",
                result: Some(Value::list(
                    vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let metadata = input.metadata();

        let args: Vec<Value> = call.rest(engine_state, stack, 0)?;
        if args.is_empty() {
            return Ok(input);
        }

        let (rows_to_drop, min_unbounded_start) = get_rows_to_drop(&args, head)?;

        let input = if let Some(cutoff) = min_unbounded_start {
            input
                .into_iter()
                .take(cutoff)
                .into_pipeline_data_with_metadata(
                    head,
                    engine_state.signals().clone(),
                    metadata.clone(),
                )
        } else {
            input
        };

        Ok(DropNthIterator {
            input: input.into_iter(),
            rows: rows_to_drop,
            current: 0,
        }
        .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

fn get_rows_to_drop(
    args: &[Value],
    head: Span,
) -> Result<(VecDeque<usize>, Option<usize>), ShellError> {
    let mut rows_to_drop = Vec::new();
    let mut min_unbounded_start: Option<usize> = None;

    for value in args {
        if let Ok(i) = value.as_int() {
            if i < 0 {
                return Err(ShellError::UnsupportedInput {
                    msg: "drop nth accepts only positive ints".into(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: value.span(),
                });
            }
            rows_to_drop.push(i as usize);
        } else if let Ok(range) = value.as_range() {
            match range {
                Range::IntRange(range) => {
                    let start = range.start();
                    if start < 0 {
                        return Err(ShellError::UnsupportedInput {
                            msg: "drop nth accepts only positive ints".into(),
                            input: "value originates from here".into(),
                            msg_span: head,
                            input_span: value.span(),
                        });
                    }

                    match range.end() {
                        Bound::Included(end) => {
                            if end < start {
                                return Err(ShellError::UnsupportedInput {
                                    msg: "The upper bound must be greater than or equal to the lower bound".into(),
                                    input: "value originates from here".into(),
                                    msg_span: head,
                                    input_span: value.span(),
                                });
                            }
                            rows_to_drop.extend((start as usize)..=(end as usize));
                        }
                        Bound::Excluded(end) => {
                            if end <= start {
                                return Err(ShellError::UnsupportedInput {
                                    msg: "The upper bound must be greater than the lower bound"
                                        .into(),
                                    input: "value originates from here".into(),
                                    msg_span: head,
                                    input_span: value.span(),
                                });
                            }
                            rows_to_drop.extend((start as usize)..(end as usize));
                        }
                        Bound::Unbounded => {
                            let start_usize = start as usize;
                            min_unbounded_start = Some(
                                min_unbounded_start.map_or(start_usize, |s| s.min(start_usize)),
                            );
                        }
                    }
                }
                Range::FloatRange(_) => {
                    return Err(ShellError::UnsupportedInput {
                        msg: "float range not supported".into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: value.span(),
                    });
                }
            }
        } else {
            return Err(ShellError::TypeMismatch {
                err_message: "Expected int or range".into(),
                span: value.span(),
            });
        }
    }

    rows_to_drop.sort_unstable();
    rows_to_drop.dedup();

    Ok((VecDeque::from(rows_to_drop), min_unbounded_start))
}

struct DropNthIterator {
    input: PipelineIterator,
    rows: VecDeque<usize>,
    current: usize,
}

impl Iterator for DropNthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.rows.front() {
                if self.current == *row {
                    self.rows.pop_front();
                    self.current += 1;
                    let _ = self.input.next();
                    continue;
                } else {
                    self.current += 1;
                    return self.input.next();
                }
            } else {
                return self.input.next();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(DropNth {})
    }
}
