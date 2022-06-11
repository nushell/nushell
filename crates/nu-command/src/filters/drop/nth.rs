use indexmap::IndexSet;
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
                example: "[0,1,2,3,4,5] | drop nth 1..3",
                description: "Drop rows 2, 3, and 4",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(4), Value::test_int(5)],
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
                description: "Drop range rows from second to fourth",
                example: "[first second third fourth fifth] | drop nth (1..3)",
                result: Some(Value::List {
                    vals: vec![Value::test_string("first"), Value::test_string("fifth")],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[0,1,2,3,4,5] | drop nth 1..3",
                description: "Drop rows 2, 3, and 4",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(4), Value::test_int(5)],
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
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let input_value = input.into_value(call.span());
        match input_value {
            Value::List { vals, span: _ } => {
                let rows = rows_to_remove(engine_state, stack, call, vals.len())?;
                let mut new_vals = vec![];
                for (idx, e) in vals.iter().enumerate() {
                    // don't "copy" a value whose index is one of the indexes of the values we want to drop
                    if !rows.contains(&idx) {
                        new_vals.push(e.clone());
                    }
                }
                Ok(new_vals.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            Value::Range { val, span: _ } => {
                let clone = val.clone();
                let input_length = clone.into_range_iter(engine_state.ctrlc.clone())?.count();
                let rows = rows_to_remove(engine_state, stack, call, input_length)?;
                let new_vals = val
                    .into_range_iter(engine_state.ctrlc.clone())?
                    .enumerate()
                    .filter(|(idx, ..)| !rows.contains(idx))
                    .map(|x| x.1)
                    .collect::<Vec<_>>();

                Ok(new_vals.into_pipeline_data(engine_state.ctrlc.clone()))
            }
            _ => Err(ShellError::UnsupportedInput(
                "Drop nth works only on lists, tables, or ranges".to_string(),
                call.head,
            )),
        }
    }
}

fn rows_to_remove(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input_size: usize,
) -> Result<IndexSet<usize>, ShellError> {
    let number_or_range = extract_int_or_range(engine_state, stack, call)?;
    // get a vector of indexes to remove
    let rows = match number_or_range {
        Either::Left(row_number) => {
            let and_rows: Vec<Spanned<i64>> = call.rest(engine_state, stack, 1)?;
            let mut rows: indexmap::IndexSet<_> =
                and_rows.into_iter().map(|x| x.item as usize).collect();
            rows.insert(row_number as usize);
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
                (from..=to).collect::<indexmap::IndexSet<_>>()
            } else {
                (from..to).collect::<indexmap::IndexSet<_>>()
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(DropNth {})
    }
}
