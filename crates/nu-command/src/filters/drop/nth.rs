use itertools::Either;
use nu_engine::command_prelude::*;
use nu_protocol::{PipelineIterator, Range};
use std::ops::Bound;

#[derive(Clone)]
pub struct DropNth;

impl Command for DropNth {
    fn name(&self) -> &str {
        "drop nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop nth")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .required(
                "row number or row range",
                // FIXME: we can make this accept either Int or Range when we can compose SyntaxShapes
                SyntaxShape::Any,
                "The number of the row to drop or a range to drop consecutive rows.",
            )
            .rest("rest", SyntaxShape::Any, "The number of the row to drop.")
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
        let number_or_range = extract_int_or_range(engine_state, stack, call)?;

        let rows = match number_or_range.item {
            Either::Left(row_number) => {
                let and_rows: Vec<Spanned<i64>> = call.rest(engine_state, stack, 1)?;
                let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
                rows.push(row_number as usize);
                rows.sort_unstable();
                rows
            }
            Either::Right(Range::FloatRange(_)) => {
                return Err(ShellError::UnsupportedInput {
                    msg: "float range".into(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: number_or_range.span,
                });
            }
            Either::Right(Range::IntRange(range)) => {
                // check for negative range inputs, e.g., (2..-5)
                let end_negative = match range.end() {
                    Bound::Included(end) | Bound::Excluded(end) => end < 0,
                    Bound::Unbounded => false,
                };
                if range.start().is_negative() || end_negative {
                    return Err(ShellError::UnsupportedInput {
                        msg: "drop nth accepts only positive ints".into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: number_or_range.span,
                    });
                }
                // check if the upper bound is smaller than the lower bound, e.g., do not accept 4..2
                if range.step() < 0 {
                    return Err(ShellError::UnsupportedInput {
                        msg: "The upper bound needs to be equal or larger to the lower bound"
                            .into(),
                        input: "value originates from here".into(),
                        msg_span: head,
                        input_span: number_or_range.span,
                    });
                }

                let start = range.start() as usize;

                let end = match range.end() {
                    Bound::Included(end) => end as usize,
                    Bound::Excluded(end) => (end - 1) as usize,
                    Bound::Unbounded => {
                        return Ok(input
                            .into_iter()
                            .take(start)
                            .into_pipeline_data_with_metadata(
                                head,
                                engine_state.signals().clone(),
                                metadata,
                            ))
                    }
                };

                let end = if let PipelineData::Value(Value::List { vals, .. }, _) = &input {
                    end.min(vals.len() - 1)
                } else {
                    end
                };

                (start..=end).collect()
            }
        };

        Ok(DropNthIterator {
            input: input.into_iter(),
            rows,
            current: 0,
        }
        .into_pipeline_data_with_metadata(head, engine_state.signals().clone(), metadata))
    }
}

fn extract_int_or_range(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Spanned<Either<i64, Range>>, ShellError> {
    let value: Value = call.req(engine_state, stack, 0)?;

    let int_opt = value.as_int().map(Either::Left).ok();
    let range_opt = value.as_range().map(Either::Right).ok();

    int_opt
        .or(range_opt)
        .ok_or_else(|| ShellError::TypeMismatch {
            err_message: "int or range".into(),
            span: value.span(),
        })
        .map(|either| Spanned {
            item: either,
            span: value.span(),
        })
}

struct DropNthIterator {
    input: PipelineIterator,
    rows: Vec<usize>,
    current: usize,
}

impl Iterator for DropNthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.rows.first() {
                if self.current == *row {
                    self.rows.remove(0);
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
