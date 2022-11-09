use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

#[derive(Clone)]
pub struct Rotate;

impl Command for Rotate {
    fn name(&self) -> &str {
        "rotate"
    }

    fn signature(&self) -> Signature {
        Signature::build("rotate")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .switch("ccw", "rotate counter clockwise", None)
            .rest(
                "rest",
                SyntaxShape::String,
                "the names to give columns once rotated",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Rotates a table clockwise (default) or counter-clockwise (use --ccw flag)."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Rotate 2x2 table clockwise",
                example: "[[a b]; [1 2]] | rotate",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_int(1), Value::test_string("a")],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_int(2), Value::test_string("b")],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate 2x3 table clockwise",
                example: "[[a b]; [1 2] [3 4] [5 6]] | rotate",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec![
                                "column0".to_string(),
                                "column1".to_string(),
                                "column2".to_string(),
                                "column3".to_string(),
                            ],
                            vals: vec![
                                Value::test_int(5),
                                Value::test_int(3),
                                Value::test_int(1),
                                Value::test_string("a"),
                            ],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec![
                                "column0".to_string(),
                                "column1".to_string(),
                                "column2".to_string(),
                                "column3".to_string(),
                            ],
                            vals: vec![
                                Value::test_int(6),
                                Value::test_int(4),
                                Value::test_int(2),
                                Value::test_string("b"),
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate table clockwise and change columns names",
                example: "[[a b]; [1 2]] | rotate col_a col_b",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["col_a".to_string(), "col_b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_string("a")],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["col_a".to_string(), "col_b".to_string()],
                            vals: vec![Value::test_int(2), Value::test_string("b")],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate table counter clockwise",
                example: "[[a b]; [1 2]] | rotate --ccw",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(2)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("a"), Value::test_int(1)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate table counter-clockwise",
                example: "[[a b]; [1 2] [3 4] [5 6]] | rotate --ccw",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec![
                                "column0".to_string(),
                                "column1".to_string(),
                                "column2".to_string(),
                                "column3".to_string(),
                            ],
                            vals: vec![
                                Value::test_string("b"),
                                Value::test_int(2),
                                Value::test_int(4),
                                Value::test_int(6),
                            ],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec![
                                "column0".to_string(),
                                "column1".to_string(),
                                "column2".to_string(),
                                "column3".to_string(),
                            ],
                            vals: vec![
                                Value::test_string("a"),
                                Value::test_int(1),
                                Value::test_int(3),
                                Value::test_int(5),
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate table counter-clockwise and change columns names",
                example: "[[a b]; [1 2]] | rotate --ccw col_a col_b",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["col_a".to_string(), "col_b".to_string()],
                            vals: vec![Value::test_string("b"), Value::test_int(2)],
                            span: Span::test_data(),
                        },
                        Value::Record {
                            cols: vec!["col_a".to_string(), "col_b".to_string()],
                            vals: vec![Value::test_string("a"), Value::test_int(1)],
                            span: Span::test_data(),
                        },
                    ],
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
    let mut values = input.into_iter().collect::<Vec<_>>();
    let mut old_column_names = vec![];
    let mut new_values = vec![];
    let mut not_a_record = false;
    let total_rows = &mut values.len();
    let ccw: bool = call.has_flag("ccw");

    if !ccw {
        values.reverse();
    }

    if !values.is_empty() {
        for val in values.into_iter() {
            match val {
                Value::Record {
                    cols,
                    vals,
                    span: _,
                } => {
                    old_column_names = cols;
                    for v in vals {
                        new_values.push(v)
                    }
                }
                Value::List { vals, span: _ } => {
                    not_a_record = true;
                    for v in vals {
                        new_values.push(v);
                    }
                }
                Value::String { val, span } => {
                    not_a_record = true;
                    new_values.push(Value::String { val, span })
                }
                x => {
                    not_a_record = true;
                    new_values.push(x)
                }
            }
        }
    } else {
        return Err(ShellError::UnsupportedInput(
            "Rotate command requires a Nu value as input".to_string(),
            call.head,
        ));
    }

    let total_columns = &old_column_names.len();

    // we use this for building columns names, but for non-records we get an extra row so we remove it
    if *total_columns == 0 {
        *total_rows -= 1;
    }

    // holder for the new column names, particularly if none are provided by the user we create names as column0, column1, etc.
    let mut new_column_names = {
        let mut res = vec![];
        for idx in 0..(*total_rows + 1) {
            res.push(format!("column{}", idx));
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
        return Ok(Value::List {
            vals: vec![Value::Record {
                cols: new_column_names,
                vals: new_values,
                span: call.head,
            }],
            span: call.head,
        }
        .into_pipeline_data()
        .set_metadata(metadata));
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
            vec![Value::String {
                val: val.to_string(),
                span: call.head,
            }]
        } else {
            vec![]
        };

        let new_vals = {
            // move through the array with a step, which is every new_values size / total rows, starting from our old column's index
            // so if initial data was like this [[a b]; [1 2] [3 4]] - we basically iterate on this [3 4 1 2] array, so we pick 3, then 1, and then when idx increases, we pick 4 and 2
            for i in (idx..new_values.len()).step_by(new_values.len() / *total_rows) {
                res.push(new_values[i].clone());
            }
            // when rotating clockwise, the old column names become the last column's values
            if !ccw {
                res.push(Value::String {
                    val: val.to_string(),
                    span: call.head,
                });
            }
            res.to_vec()
        };
        final_values.push(Value::Record {
            cols: new_column_names.clone(),
            vals: new_vals,
            span: call.head,
        })
    }

    Ok(Value::List {
        vals: final_values,
        span: call.head,
    }
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
