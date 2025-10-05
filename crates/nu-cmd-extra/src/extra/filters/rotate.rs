use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Rotate;

impl Command for Rotate {
    fn name(&self) -> &str {
        "rotate"
    }

    fn signature(&self) -> Signature {
        Signature::build("rotate")
            .input_output_types(vec![
                (Type::record(), Type::table()),
                (Type::table(), Type::table()),
                (Type::list(Type::Any), Type::table()),
                (Type::String, Type::table()),
            ])
            .switch("ccw", "rotate counter clockwise", None)
            .rest(
                "rest",
                SyntaxShape::String,
                "The names to give columns once rotated.",
            )
            .category(Category::Filters)
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Rotates a table or record clockwise (default) or counter-clockwise (use --ccw flag)."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Rotate a record clockwise, producing a table (like `transpose` but with column order reversed)",
                example: "{a:1, b:2} | rotate",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_int(1),
                        "column1" => Value::test_string("a"),
                    }),
                    Value::test_record(record! {
                        "column0" => Value::test_int(2),
                        "column1" => Value::test_string("b"),
                    }),
                ])),
            },
            Example {
                description: "Rotate 2x3 table clockwise",
                example: "[[a b]; [1 2] [3 4] [5 6]] | rotate",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_int(5),
                        "column1" => Value::test_int(3),
                        "column2" => Value::test_int(1),
                        "column3" => Value::test_string("a"),
                    }),
                    Value::test_record(record! {
                        "column0" => Value::test_int(6),
                        "column1" => Value::test_int(4),
                        "column2" => Value::test_int(2),
                        "column3" => Value::test_string("b"),
                    }),
                ])),
            },
            Example {
                description: "Rotate table clockwise and change columns names",
                example: "[[a b]; [1 2]] | rotate col_a col_b",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "col_a" => Value::test_int(1),
                        "col_b" => Value::test_string("a"),
                    }),
                    Value::test_record(record! {
                        "col_a" => Value::test_int(2),
                        "col_b" => Value::test_string("b"),
                    }),
                ])),
            },
            Example {
                description: "Rotate table counter clockwise",
                example: "[[a b]; [1 2]] | rotate --ccw",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_string("b"),
                        "column1" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "column0" => Value::test_string("a"),
                        "column1" => Value::test_int(1),
                    }),
                ])),
            },
            Example {
                description: "Rotate table counter-clockwise",
                example: "[[a b]; [1 2] [3 4] [5 6]] | rotate --ccw",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "column0" => Value::test_string("b"),
                        "column1" => Value::test_int(2),
                        "column2" => Value::test_int(4),
                        "column3" => Value::test_int(6),
                    }),
                    Value::test_record(record! {
                        "column0" => Value::test_string("a"),
                        "column1" => Value::test_int(1),
                        "column2" => Value::test_int(3),
                        "column3" => Value::test_int(5),
                    }),
                ])),
            },
            Example {
                description: "Rotate table counter-clockwise and change columns names",
                example: "[[a b]; [1 2]] | rotate --ccw col_a col_b",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "col_a" => Value::test_string("b"),
                        "col_b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "col_a" => Value::test_string("a"),
                        "col_b" => Value::test_int(1),
                    }),
                ])),
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
        rotate(engine_state, stack, call, input)
    }
}

pub fn rotate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let metadata = input.metadata();
    let col_given_names: Vec<String> = call.rest(engine_state, stack, 0)?;
    let input_span = input.span().unwrap_or(call.head);
    let mut values = input.into_iter().collect::<Vec<_>>();
    let mut old_column_names = vec![];
    let mut new_values = vec![];
    let mut not_a_record = false;
    let mut total_rows = values.len();
    let ccw: bool = call.has_flag(engine_state, stack, "ccw")?;

    if !ccw {
        values.reverse();
    }

    if !values.is_empty() {
        for val in values.into_iter() {
            let span = val.span();
            match val {
                Value::Record { val: record, .. } => {
                    let (cols, vals): (Vec<_>, Vec<_>) = record.into_owned().into_iter().unzip();
                    old_column_names = cols;
                    new_values.extend_from_slice(&vals);
                }
                Value::List { vals, .. } => {
                    not_a_record = true;
                    for v in vals {
                        new_values.push(v);
                    }
                }
                Value::String { val, .. } => {
                    not_a_record = true;
                    new_values.push(Value::string(val, span))
                }
                x => {
                    not_a_record = true;
                    new_values.push(x)
                }
            }
        }
    } else {
        return Err(ShellError::UnsupportedInput {
            msg: "list input is empty".to_string(),
            input: "value originates from here".into(),
            msg_span: call.head,
            input_span,
        });
    }

    let total_columns = old_column_names.len();

    // we use this for building columns names, but for non-records we get an extra row so we remove it
    if total_columns == 0 {
        total_rows -= 1;
    }

    // holder for the new column names, particularly if none are provided by the user we create names as column0, column1, etc.
    let mut new_column_names = {
        let mut res = vec![];
        for idx in 0..(total_rows + 1) {
            res.push(format!("column{idx}"));
        }
        res.to_vec()
    };

    // we got new names for columns from the input, so we need to swap those we already made
    if !col_given_names.is_empty() {
        for (idx, val) in col_given_names.into_iter().enumerate() {
            if idx > new_column_names.len() - 1 {
                break;
            }
            new_column_names[idx] = val;
        }
    }

    if not_a_record {
        let record =
            Record::from_raw_cols_vals(new_column_names, new_values, input_span, call.head)?;

        return Ok(
            Value::list(vec![Value::record(record, call.head)], call.head)
                .into_pipeline_data()
                .set_metadata(metadata),
        );
    }

    // holder for the new records
    let mut final_values = vec![];

    // the number of initial columns will be our number of rows, so we iterate through that to get the new number of rows that we need to make
    // for counter clockwise, we're iterating from right to left and have a pair of (index, value)
    let columns_iter = if ccw {
        old_column_names
            .iter()
            .enumerate()
            .rev()
            .collect::<Vec<_>>()
    } else {
        // as we're rotating clockwise, we're iterating from left to right and have a pair of (index, value)
        old_column_names.iter().enumerate().collect::<Vec<_>>()
    };

    for (idx, val) in columns_iter {
        // when rotating counter clockwise, the old columns names become the first column's values
        let mut res = if ccw {
            vec![Value::string(val, call.head)]
        } else {
            vec![]
        };

        let new_vals = {
            // move through the array with a step, which is every new_values size / total rows, starting from our old column's index
            // so if initial data was like this [[a b]; [1 2] [3 4]] - we basically iterate on this [3 4 1 2] array, so we pick 3, then 1, and then when idx increases, we pick 4 and 2
            for i in (idx..new_values.len()).step_by(new_values.len() / total_rows) {
                res.push(new_values[i].clone());
            }
            // when rotating clockwise, the old column names become the last column's values
            if !ccw {
                res.push(Value::string(val, call.head));
            }
            res.to_vec()
        };

        let record =
            Record::from_raw_cols_vals(new_column_names.clone(), new_vals, input_span, call.head)?;

        final_values.push(Value::record(record, call.head))
    }

    Ok(Value::list(final_values, call.head)
        .into_pipeline_data()
        .set_metadata(metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Rotate)
    }
}
