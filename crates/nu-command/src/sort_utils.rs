use alphanumeric_sort::compare_str;
use nu_engine::column::nonexistent_column;
use nu_protocol::{ShellError, Span, Value};
use nu_utils::IgnoreCaseExt;
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
    let span = val.span();
    match val {
        Value::List { vals, .. } => {
            let mut vals = vals.clone();
            sort(&mut vals, sort_columns, span, insensitive, natural)?;

            if !ascending {
                vals.reverse();
            }

            Ok(Value::list(vals, span))
        }
        Value::Custom { val, .. } => {
            let base_val = val.to_base_value(span)?;
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
    let span = val.span();
    if let Value::List { vals, .. } = val {
        sort(vals, sort_columns, span, insensitive, natural)?;
        if !ascending {
            vals.reverse();
        }
    }
    Ok(())
}

pub fn sort(
    vec: &mut [Value],
    sort_columns: Vec<String>,
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    let val_span = vec.first().map(|v| v.span()).unwrap_or(span);
    match vec.first() {
        Some(Value::Record { val: record, .. }) => {
            if sort_columns.is_empty() {
                // This uses the same format as the 'requires a column name' error in split_by.rs
                return Err(ShellError::GenericError {
                    error: "expected name".into(),
                    msg: "requires a column name to sort table data".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }

            if let Some(nonexistent) = nonexistent_column(&sort_columns, record.columns()) {
                return Err(ShellError::CantFindColumn {
                    col_name: nonexistent,
                    span,
                    src_span: val_span,
                });
            }

            // check to make sure each value in each column in the record
            // that we asked for is a string. So, first collect all the columns
            // that we asked for into vals, then later make sure they're all
            // strings.
            let mut vals = vec![];
            for item in vec.iter() {
                for col in &sort_columns {
                    let val = item
                        .get_data_by_key(col)
                        .unwrap_or_else(|| Value::nothing(Span::unknown()));
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
                    let span_a = a.span();
                    let span_b = b.span();
                    let folded_left = match a {
                        Value::String { val, .. } => Value::string(val.to_folded_case(), span_a),
                        _ => a.clone(),
                    };

                    let folded_right = match b {
                        Value::String { val, .. } => Value::string(val.to_folded_case(), span_b),
                        _ => b.clone(),
                    };

                    if natural {
                        match (
                            folded_left.coerce_into_string(),
                            folded_right.coerce_into_string(),
                        ) {
                            (Ok(left), Ok(right)) => compare_str(left, right),
                            _ => Ordering::Equal,
                        }
                    } else {
                        folded_left
                            .partial_cmp(&folded_right)
                            .unwrap_or(Ordering::Equal)
                    }
                } else if natural {
                    match (a.coerce_str(), b.coerce_str()) {
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
            None => Value::nothing(span),
        };

        let right_value = right.get_data_by_key(column);

        let right_res = match right_value {
            Some(right_res) => right_res,
            None => Value::nothing(span),
        };

        let result = if insensitive {
            let span_left = left_res.span();
            let span_right = right_res.span();
            let folded_left = match left_res {
                Value::String { val, .. } => Value::string(val.to_folded_case(), span_left),
                _ => left_res,
            };

            let folded_right = match right_res {
                Value::String { val, .. } => Value::string(val.to_folded_case(), span_right),
                _ => right_res,
            };
            if natural {
                match (
                    folded_left.coerce_into_string(),
                    folded_right.coerce_into_string(),
                ) {
                    (Ok(left), Ok(right)) => compare_str(left, right),
                    _ => Ordering::Equal,
                }
            } else {
                folded_left
                    .partial_cmp(&folded_right)
                    .unwrap_or(Ordering::Equal)
            }
        } else if natural {
            match (
                left_res.coerce_into_string(),
                right_res.coerce_into_string(),
            ) {
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
mod tests {
    use super::*;
    use nu_protocol::{record, Value};

    #[test]
    fn test_sort_value() {
        let val = Value::test_list(vec![
            Value::test_record(record! {
            "fruit" => Value::test_string("pear"),
            "count" => Value::test_int(3),
            }),
            Value::test_record(record! {
            "fruit" => Value::test_string("orange"),
            "count" => Value::test_int(7),
            }),
            Value::test_record(record! {
            "fruit" => Value::test_string("apple"),
            "count" => Value::test_int(9),
            }),
        ]);

        let sorted_alphabetically =
            sort_value(&val, vec!["fruit".to_string()], true, false, false).unwrap();
        assert_eq!(
            sorted_alphabetically,
            Value::test_list(vec![
                Value::test_record(record! {
                "fruit" => Value::test_string("apple"),
                "count" => Value::test_int(9),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("orange"),
                "count" => Value::test_int(7),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("pear"),
                "count" => Value::test_int(3),
                            }),
            ],)
        );

        let sorted_by_count_desc =
            sort_value(&val, vec!["count".to_string()], false, false, false).unwrap();
        assert_eq!(
            sorted_by_count_desc,
            Value::test_list(vec![
                Value::test_record(record! {
                "fruit" => Value::test_string("apple"),
                "count" => Value::test_int(9),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("orange"),
                "count" => Value::test_int(7),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("pear"),
                "count" => Value::test_int(3),
                            }),
            ],)
        );
    }

    #[test]
    fn test_sort_value_in_place() {
        let mut val = Value::test_list(vec![
            Value::test_record(record! {
            "fruit" => Value::test_string("pear"),
            "count" => Value::test_int(3),
            }),
            Value::test_record(record! {
            "fruit" => Value::test_string("orange"),
            "count" => Value::test_int(7),
            }),
            Value::test_record(record! {
            "fruit" => Value::test_string("apple"),
            "count" => Value::test_int(9),
            }),
        ]);

        sort_value_in_place(&mut val, vec!["fruit".to_string()], true, false, false).unwrap();
        assert_eq!(
            val,
            Value::test_list(vec![
                Value::test_record(record! {
                "fruit" => Value::test_string("apple"),
                "count" => Value::test_int(9),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("orange"),
                "count" => Value::test_int(7),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("pear"),
                "count" => Value::test_int(3),
                            }),
            ],)
        );

        sort_value_in_place(&mut val, vec!["count".to_string()], false, false, false).unwrap();
        assert_eq!(
            val,
            Value::test_list(vec![
                Value::test_record(record! {
                "fruit" => Value::test_string("apple"),
                "count" => Value::test_int(9),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("orange"),
                "count" => Value::test_int(7),
                            }),
                Value::test_record(record! {
                "fruit" => Value::test_string("pear"),
                "count" => Value::test_int(3),
                            }),
            ],)
        );
    }
}
