use alphanumeric_sort::compare_str;
use nu_engine::column::column_does_not_exist;
use nu_protocol::{ShellError, Span, Value};
use std::cmp::Ordering;

// This module includes sorting functionality that is useful in sort-by and elsewhere.
// Eventually it would be nice to find a better home for it; sorting logic is only coupled
// to commands for historical reasons.

/// Sort a value. This only makes sense for lists and list-like things,
/// so for everything else we just return the value as-is.
/// CustomValues are converted to their base value and then sorted.
pub fn sort_value(
    val: &Value,
    sort_columns: Vec<String>,
    ascending: bool,
    insensitive: bool,
    natural: bool,
) -> Result<Value, ShellError> {
    match val {
        Value::List { vals, span } => {
            let mut vals = vals.clone();
            sort(&mut vals, sort_columns, *span, insensitive, natural)?;

            if !ascending {
                vals.reverse();
            }

            Ok(Value::List { vals, span: *span })
        }
        Value::CustomValue { val, span } => {
            let base_val = val.to_base_value(*span)?;
            sort_value(&base_val, sort_columns, ascending, insensitive, natural)
        }
        _ => Ok(val.to_owned()),
    }
}

/// Sort a value in-place. This is more efficient than sort_value() because it
/// avoids cloning, but it does not work for CustomValues; they are returned as-is.
pub fn sort_value_in_place(
    val: &mut Value,
    sort_columns: Vec<String>,
    ascending: bool,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    match val {
        Value::List { vals, span } => {
            sort(vals, sort_columns, *span, insensitive, natural)?;
            if !ascending {
                vals.reverse();
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn sort(
    vec: &mut [Value],
    sort_columns: Vec<String>,
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
            if sort_columns.is_empty() {
                println!("sort-by requires a column name to sort table data");
                return Err(ShellError::CantFindColumn(span, span));
            }

            if column_does_not_exist(sort_columns.clone(), cols.to_vec()) {
                return Err(ShellError::CantFindColumn(span, span));
            }

            // check to make sure each value in each column in the record
            // that we asked for is a string. So, first collect all the columns
            // that we asked for into vals, then later make sure they're all
            // strings.
            let mut vals = vec![];
            for item in vec.iter() {
                for col in &sort_columns {
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
                compare(
                    a,
                    b,
                    &sort_columns,
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

pub fn compare(
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

#[test]
fn test_sort_value() {
    let val = Value::List {
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
    };

    let sorted_alphabetically =
        sort_value(&val, vec!["fruit".to_string()], true, false, false).unwrap();
    assert_eq!(
        sorted_alphabetically,
        Value::List {
            vals: vec![
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("apple"), Value::test_int(9)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("orange"), Value::test_int(7)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("pear"), Value::test_int(3)],
                ),
            ],
            span: Span::test_data(),
        }
    );

    let sorted_by_count_desc =
        sort_value(&val, vec!["count".to_string()], false, false, false).unwrap();
    assert_eq!(
        sorted_by_count_desc,
        Value::List {
            vals: vec![
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("apple"), Value::test_int(9)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("orange"), Value::test_int(7)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("pear"), Value::test_int(3)],
                ),
            ],
            span: Span::test_data(),
        }
    );
}

#[test]
fn test_sort_value_in_place() {
    let mut val = Value::List {
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
    };

    sort_value_in_place(&mut val, vec!["fruit".to_string()], true, false, false).unwrap();
    assert_eq!(
        val,
        Value::List {
            vals: vec![
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("apple"), Value::test_int(9)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("orange"), Value::test_int(7)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("pear"), Value::test_int(3)],
                ),
            ],
            span: Span::test_data(),
        }
    );

    sort_value_in_place(&mut val, vec!["count".to_string()], false, false, false).unwrap();
    assert_eq!(
        val,
        Value::List {
            vals: vec![
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("apple"), Value::test_int(9)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("orange"), Value::test_int(7)],
                ),
                Value::test_record(
                    vec!["fruit", "count"],
                    vec![Value::test_string("pear"), Value::test_int(3)],
                ),
            ],
            span: Span::test_data(),
        }
    );
}
