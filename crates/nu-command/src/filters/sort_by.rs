use alphanumeric_sort::compare_str;
use nu_engine::{column::column_does_not_exist, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Value,
};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct SortBy;

impl Command for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("sort-by")
            .rest("columns", SyntaxShape::Any, "the column(s) to sort by")
            .switch("reverse", "Sort in reverse order", Some('r'))
            .switch(
                "insensitive",
                "Sort string-based columns case-insensitively",
                Some('i'),
            )
            .switch(
                "natural",
                "Sort alphanumeric string-based columns naturally",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[2 0 1] | sort-by",
                description: "sort the list by increasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[2 0 1] | sort-by -r",
                description: "sort the list by decreasing value",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(1), Value::test_int(0)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort-by",
                description: "sort a list of strings",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("amy"),
                        Value::test_string("betty"),
                        Value::test_string("sarah"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[betty amy sarah] | sort-by -r",
                description: "sort a list of strings in reverse",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("sarah"),
                        Value::test_string("betty"),
                        Value::test_string("amy"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[test1 test11 test2] | sort-by -n",
                description: "sort a list of alphanumeric strings naturally",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("test1"),
                        Value::test_string("test2"),
                        Value::test_string("test11"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort strings (case-insensitive)",
                example: "echo [airplane Truck Car] | sort-by -i",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("airplane"),
                        Value::test_string("Car"),
                        Value::test_string("Truck"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort strings (reversed case-insensitive)",
                example: "echo [airplane Truck Car] | sort-by -i -r",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("Truck"),
                        Value::test_string("Car"),
                        Value::test_string("airplane"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Sort a table by its column (reversed order)",
                example: "[[fruit count]; [apple 9] [pear 3] [orange 7]] | sort-by fruit -r",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("pear"), Value::test_int(3)],
                        ),
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("orange"), Value::test_int(7)],
                        ),
                        Value::test_record(
                            vec!["fruit", "count"],
                            vec![Value::test_string("apple"), Value::test_int(9)],
                        ),
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
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
        let reverse = call.has_flag("reverse");
        let insensitive = call.has_flag("insensitive");
        let natural = call.has_flag("natural");
        let metadata = &input.metadata();
        let mut vec: Vec<_> = input.into_iter().collect();

        sort(&mut vec, columns, call.head, insensitive, natural)?;

        if reverse {
            vec.reverse()
        }

        let iter = vec.into_iter();
        match &*metadata {
            Some(m) => {
                Ok(iter.into_pipeline_data_with_metadata(m.clone(), engine_state.ctrlc.clone()))
            }
            None => Ok(iter.into_pipeline_data(engine_state.ctrlc.clone())),
        }
    }
}

pub fn sort(
    vec: &mut [Value],
    columns: Vec<String>,
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    if vec.is_empty() {
        return Err(ShellError::GenericError(
            "no values to work with".to_string(),
            "".to_string(),
            None,
            Some("no values to work with".to_string()),
            Vec::new(),
        ));
    }

    match &vec[0] {
        Value::Record {
            cols,
            vals: _input_vals,
            ..
        } => {
            if columns.is_empty() {
                println!("sort-by requires a column name to sort table data");
                return Err(ShellError::CantFindColumn(span, span));
            }

            if column_does_not_exist(columns.clone(), cols.to_vec()) {
                return Err(ShellError::CantFindColumn(span, span));
            }

            // check to make sure each value in each column in the record
            // that we asked for is a string. So, first collect all the columns
            // that we asked for into vals, then later make sure they're all
            // strings.
            let mut vals = vec![];
            for item in vec.iter() {
                for col in &columns {
                    let val = match item.get_data_by_key(col) {
                        Some(v) => v,
                        None => Value::nothing(Span::test_data()),
                    };
                    vals.push(val);
                }
            }

            let should_sort_case_insensitively = insensitive
                && vals
                    .iter()
                    .all(|x| matches!(x.get_type(), nu_protocol::Type::String));

            let should_sort_case_naturally = natural
                && vals
                    .iter()
                    .all(|x| matches!(x.get_type(), nu_protocol::Type::String));

            vec.sort_by(|a, b| {
                process(
                    a,
                    b,
                    &columns,
                    span,
                    should_sort_case_insensitively,
                    should_sort_case_naturally,
                )
            });
        }
        _ => {
            vec.sort_by(|a, b| {
                if insensitive {
                    let lowercase_left = match a {
                        Value::String { val, span } => Value::String {
                            val: val.to_ascii_lowercase(),
                            span: *span,
                        },
                        _ => a.clone(),
                    };

                    let lowercase_right = match b {
                        Value::String { val, span } => Value::String {
                            val: val.to_ascii_lowercase(),
                            span: *span,
                        },
                        _ => b.clone(),
                    };

                    if natural {
                        match (lowercase_left.as_string(), lowercase_right.as_string()) {
                            (Ok(left), Ok(right)) => compare_str(left, right),
                            _ => Ordering::Equal,
                        }
                    } else {
                        lowercase_left
                            .partial_cmp(&lowercase_right)
                            .unwrap_or(Ordering::Equal)
                    }
                } else if natural {
                    match (a.as_string(), b.as_string()) {
                        (Ok(left), Ok(right)) => compare_str(left, right),
                        _ => Ordering::Equal,
                    }
                } else {
                    a.partial_cmp(b).unwrap_or(Ordering::Equal)
                }
            });
        }
    }
    Ok(())
}

pub fn process(
    left: &Value,
    right: &Value,
    columns: &[String],
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Ordering {
    for column in columns {
        let left_value = left.get_data_by_key(column);

        let left_res = match left_value {
            Some(left_res) => left_res,
            None => Value::Nothing { span },
        };

        let right_value = right.get_data_by_key(column);

        let right_res = match right_value {
            Some(right_res) => right_res,
            None => Value::Nothing { span },
        };

        let result = if insensitive {
            let lowercase_left = match left_res {
                Value::String { val, span } => Value::String {
                    val: val.to_ascii_lowercase(),
                    span,
                },
                _ => left_res,
            };

            let lowercase_right = match right_res {
                Value::String { val, span } => Value::String {
                    val: val.to_ascii_lowercase(),
                    span,
                },
                _ => right_res,
            };
            if natural {
                match (lowercase_left.as_string(), lowercase_right.as_string()) {
                    (Ok(left), Ok(right)) => compare_str(left, right),
                    _ => Ordering::Equal,
                }
            } else {
                lowercase_left
                    .partial_cmp(&lowercase_right)
                    .unwrap_or(Ordering::Equal)
            }
        } else if natural {
            match (left_res.as_string(), right_res.as_string()) {
                (Ok(left), Ok(right)) => compare_str(left, right),
                _ => Ordering::Equal,
            }
        } else {
            left_res.partial_cmp(&right_res).unwrap_or(Ordering::Equal)
        };
        if result != Ordering::Equal {
            return result;
        }
    }

    Ordering::Equal
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SortBy {})
    }
}
