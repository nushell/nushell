use std::collections::HashSet;

use itertools::Either;
use nu_engine::CallExt;
use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, PipelineData, Range, ShellError,
    Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DropNth;

impl Command for DropNth {
    fn name(&self) -> &str {
        "drop nth"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop nth")
            .required(
                "row number or row range",
                // FIXME: we can make this accept either Int or Range when we can compose SyntaxShapes
                SyntaxShape::Any,
                "the number of the row to drop or a range to drop consecutive rows",
            )
            .rest("rest", SyntaxShape::Any, "the number of the row to drop")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Drop the selected rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[sam,sarah,2,3,4,5] | drop nth 0 1 2",
                description: "Drop the first, second, and third row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(4), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 0 1 2",
                description: "Drop the first, second, and third row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(3), Value::test_int(4), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 0 2 4",
                description: "Drop rows 0 2 4",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 2 0 4",
                description: "Drop rows 2 0 4",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Drop range rows from second to fourth",
                example: "echo [first second third fourth fifth] | drop nth (1..3)",
                result: Some(Value::List {
                    vals: vec![Value::test_string("first"), Value::test_string("fifth")],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 1..",
                description: "Drop all rows except first row",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 3..",
                description: "Drop rows 3,4,5",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
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
        match input {
            PipelineData::Value(Value::List { vals, span: _ }, ..) => {
                let rows_to_remove = rows_to_remove(engine_state, stack, call, vals.len())?;
                let rows = rows_to_remove
                    .into_iter()
                    .map(|x| x as usize)
                    .collect::<Vec<usize>>();
                Ok(DropNthIterator {
                    input: Box::new(vals.into_iter()),
                    rows,
                    current: 0,
                }
                .into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Value(
                Value::Record {
                    cols: _,
                    vals,
                    span: _,
                },
                ..,
            ) => {
                let rows_to_remove = rows_to_remove(engine_state, stack, call, vals.len())?;
                let rows = rows_to_remove
                    .into_iter()
                    .map(|x| x as usize)
                    .collect::<Vec<usize>>();
                Ok(DropNthIterator {
                    input: Box::new(vals.into_iter()),
                    rows,
                    current: 0,
                }
                .into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Value(Value::Range { val, span: _ }, ..) => {
                let range_iter = val.into_range_iter(engine_state.ctrlc.clone())?;
                let vals = range_iter.collect::<Vec<_>>();
                let rows_to_remove = rows_to_remove(engine_state, stack, call, vals.len())?;
                let rows = rows_to_remove
                    .into_iter()
                    .map(|x| x as usize)
                    .collect::<Vec<usize>>();
                Ok(DropNthIterator {
                    input: Box::new(vals.into_iter()),
                    rows,
                    current: 0,
                }
                .into_pipeline_data(engine_state.ctrlc.clone()))
            }
            PipelineData::Value(_v, ..) => Ok(PipelineData::new(call.span())),
            PipelineData::ListStream(ref _stream, ..) => {
                // check if the input gives an upper bound
                let check = check_upper_bound(engine_state, stack, call);
                match check {
                    // likely a set of indices
                    Ok(None) => {
                        let rows_to_remove = rows_to_remove(engine_state, stack, call, 0)?;
                        let rows = rows_to_remove
                            .into_iter()
                            .map(|x| x as usize)
                            .collect::<HashSet<usize>>();
                        return Ok(input
                            .into_iter()
                            .enumerate()
                            .filter_map(move |(idx, value)| {
                                if !rows.contains(&idx) {
                                    Some(value)
                                } else {
                                    None
                                }
                            })
                            .into_pipeline_data(engine_state.ctrlc.clone()));
                    }
                    Ok(Some(Bound {
                        lower_bound,
                        upper_bound: None,
                    })) => {
                        // we do not have an upper bound, thus we drop all the elements after first lower bound
                        return Ok(input
                            .into_iter()
                            .enumerate()
                            .filter_map(
                                move |(idx, value)| {
                                    if idx < lower_bound {
                                        Some(value)
                                    } else {
                                        None
                                    }
                                },
                            )
                            .into_pipeline_data(engine_state.ctrlc.clone()));
                    }
                    Ok(Some(Bound {
                        lower_bound,
                        upper_bound,
                    })) => {
                        let upper_bnd = upper_bound.unwrap_or(lower_bound);

                        return Ok(input
                            .into_iter()
                            .enumerate()
                            .filter_map(move |(idx, value)| {
                                if idx < lower_bound || idx > upper_bnd {
                                    Some(value)
                                } else {
                                    None
                                }
                            })
                            .into_pipeline_data(engine_state.ctrlc.clone()));
                    }
                    Err(e) => Err(e),
                }
            }
            PipelineData::ExternalStream {
                stdout: _,
                stderr: _,
                exit_code: _,
                span: _,
                metadata: _,
            } => todo!(),
        }
    }
}

struct Bound {
    lower_bound: usize,
    upper_bound: Option<usize>,
}

/// Check if the drop nth arg has an explicit upper bound
/// Return either a None if the args is not a range, or the lower bound + upper bound (if given)
fn check_upper_bound(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<Bound>, ShellError> {
    let number_or_range = extract_int_or_range(engine_state, stack, call)?;

    match number_or_range {
        Either::Left(_row_number) => {
            return Ok(None);
        }
        Either::Right(row_range) => {
            let from = row_range.from.as_integer()? as usize;
            let size = row_range.to.as_integer()? as isize;
            if isize::MAX == size {
                return Ok(Some(Bound {
                    lower_bound: from,
                    upper_bound: None,
                }));
            } else {
                let to = size as usize;
                return Ok(Some(Bound {
                    lower_bound: from,
                    upper_bound: Some(to),
                }));
            }
        }
    }
}

fn rows_to_remove(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input_size: usize,
) -> Result<Vec<usize>, ShellError> {
    let number_or_range = extract_int_or_range(engine_state, stack, call)?;

    // get a vector of indexes to remove
    let rows = match number_or_range {
        Either::Left(_row_number) => {
            let and_rows: Vec<Spanned<i64>> = call.rest(engine_state, stack, 0)?;
            let mut rows: Vec<_> = and_rows.into_iter().map(|x| x.item as usize).collect();
            rows.sort_unstable();
            rows
        }
        Either::Right(row_range) => {
            let from = row_range.from.as_integer()? as usize;
            let to = {
                let size = row_range.to.as_integer()? as usize;

                // if range does not have an upper bound specified, then to's value is the length of the list
                if size > input_size {
                    input_size - 1
                } else {
                    size
                }
            };
            if matches!(row_range.inclusion, RangeInclusion::Inclusive) {
                (from..=to).collect::<Vec<_>>()
            } else {
                (from..to).collect::<Vec<_>>()
            }
        }
    };

    Ok(rows)
}

fn extract_int_or_range(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Either<i64, Range>, ShellError> {
    let value = call.req::<Value>(engine_state, stack, 0)?;

    let int_opt = value.as_integer().map(Either::Left).ok();
    let range_opt: Result<nu_protocol::Range, ShellError> = FromValue::from_value(&value);

    let range_opt = range_opt.map(Either::Right).ok();

    int_opt.or(range_opt).ok_or_else(|| {
        ShellError::TypeMismatch(
            "int or range".into(),
            value.span().unwrap_or_else(|_| Span::new(0, 0)),
        )
    })
}

struct DropNthIterator {
    input: Box<dyn Iterator<Item = Value> + Send>,
    rows: Vec<usize>,
    current: usize,
}

impl Iterator for DropNthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.rows.get(0) {
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
