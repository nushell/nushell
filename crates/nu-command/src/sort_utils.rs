use alphanumeric_sort::compare_str;
use nu_engine::ClosureEval;
use nu_protocol::{
    ast::CellPath,
    engine::{Closure, EngineState, Stack},
    PipelineData, Record, ShellError, Span, Value,
};
use nu_utils::IgnoreCaseExt;
use std::cmp::Ordering;

// This module includes sorting functionality that is useful in sort-by and elsewhere.
// Eventually it would be nice to find a better home for it; sorting logic is only coupled
// to commands for historical reasons.

pub enum Comparator {
    Closure(Closure, EngineState, Stack),
    CellPath(CellPath),
}

pub fn sort(vec: &mut [Value], insensitive: bool, natural: bool) -> Result<(), ShellError> {
    // to apply insensitive or natural sorting, all values must be strings
    let string_sort: bool = vec
        .iter()
        .all(|value| matches!(value, &Value::String { .. }));

    // allow the comparator function to indicate error
    // by mutating this option captured by the closure,
    // since sort_by closure must be infallible
    let mut compare_err: Option<ShellError> = None;

    vec.sort_by(|a, b| {
        crate::compare_values(a, b, insensitive && string_sort, natural && string_sort)
            .unwrap_or_else(|err| {
                compare_err.get_or_insert(err);
                Ordering::Equal
            })
    });

    if let Some(err) = compare_err {
        Err(err)
    } else {
        Ok(())
    }
}

pub fn sort_by(
    vec: &mut [Value],
    comparators: Vec<Comparator>,
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    if comparators.is_empty() {
        // This uses the same format as the 'requires a column name' error in split_by.rs
        return Err(ShellError::GenericError {
            error: "expected name".into(),
            msg: "requires a cell path or closure to sort data".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        });
    }

    // to apply insensitive or natural sorting, all values must be strings
    let string_sort: bool = comparators.iter().all(|cmp| {
        let Comparator::CellPath(cell_path) = cmp else {
            // closures shouldn't affect whether cell paths are sorted naturally/insensitively
            return true;
        };
        vec.iter().all(|value| {
            let inner = value.clone().follow_cell_path(&cell_path.members, false);
            matches!(inner, Ok(Value::String { .. }))
        })
    });

    // allow the comparator function to indicate error
    // by mutating this option captured by the closure,
    // since sort_by closure must be infallible
    let mut compare_err: Option<ShellError> = None;

    vec.sort_by(|a, b| {
        compare_by(
            a,
            b,
            &comparators,
            span,
            insensitive && string_sort,
            natural && string_sort,
            &mut compare_err,
        )
    });

    if let Some(err) = compare_err {
        Err(err)
    } else {
        Ok(())
    }
}

pub fn sort_record(
    record: Record,
    sort_by_value: bool,
    reverse: bool,
    insensitive: bool,
    natural: bool,
) -> Result<Record, ShellError> {
    let mut input_pairs: Vec<(String, Value)> = record.into_iter().collect();

    // allow the comparator function to indicate error
    // by mutating this option captured by the closure,
    // since sort_by closure must be infallible
    let mut compare_err: Option<ShellError> = None;

    input_pairs.sort_by(|a, b| {
        if sort_by_value {
            compare_values(&a.1, &b.1, insensitive, natural).unwrap_or_else(|err| {
                compare_err.get_or_insert(err);
                Ordering::Equal
            })
        } else {
            compare_strings(&a.0, &b.0, insensitive, natural)
        }
    });

    if reverse {
        input_pairs.reverse()
    }

    if let Some(err) = compare_err {
        return Err(err);
    }

    Ok(input_pairs.into_iter().collect())
}

pub fn compare_by(
    left: &Value,
    right: &Value,
    comparators: &[Comparator],
    span: Span,
    insensitive: bool,
    natural: bool,
    error: &mut Option<ShellError>,
) -> Ordering {
    for cmp in comparators.iter() {
        let result = match cmp {
            Comparator::CellPath(cell_path) => {
                compare_cell_path(left, right, cell_path, insensitive, natural)
            }
            Comparator::Closure(closure, engine_state, stack) => {
                let closure_eval = ClosureEval::new(engine_state, stack, closure.clone());
                compare_closure(left, right, closure_eval, span)
            }
        };
        match result {
            Ok(Ordering::Equal) => {}
            Ok(ordering) => return ordering,
            Err(err) => {
                // don't bother continuing through the remaining comparators as we've hit an error
                // don't overwrite if there's an existing error
                error.get_or_insert(err);
                return Ordering::Equal;
            }
        }
    }
    Ordering::Equal
}

pub fn compare_values(
    left: &Value,
    right: &Value,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    if insensitive || natural {
        let left_str = left.coerce_string()?;
        let right_str = right.coerce_string()?;
        Ok(compare_strings(&left_str, &right_str, insensitive, natural))
    } else {
        Ok(left.partial_cmp(&right).unwrap_or(Ordering::Equal))
    }
}

pub fn compare_strings(
    left: &String,
    right: &String,
    insensitive: bool,
    natural: bool,
) -> Ordering {
    // declare these names now to appease compiler
    // not needed in nightly, but needed as of 1.77.2, so can be removed later
    let (left_copy, right_copy);

    // only allocate new String if necessary for case folding,
    // so callers don't need to pass an owned String
    let (left_str, right_str) = if insensitive {
        left_copy = left.to_folded_case();
        right_copy = right.to_folded_case();
        (&left_copy, &right_copy)
    } else {
        (left, right)
    };

    if natural {
        alphanumeric_sort::compare_str(left_str, right_str)
    } else {
        left_str.partial_cmp(right_str).unwrap_or(Ordering::Equal)
    }
}

pub fn compare_cell_path(
    left: &Value,
    right: &Value,
    cell_path: &CellPath,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    let left = left.clone().follow_cell_path(&cell_path.members, false)?;
    let right = right.clone().follow_cell_path(&cell_path.members, false)?;
    compare_values(&left, &right, insensitive, natural)
}

pub fn compare_closure(
    left: &Value,
    right: &Value,
    mut closure_eval: ClosureEval,
    span: Span,
) -> Result<Ordering, ShellError> {
    closure_eval
        .add_arg(left.clone())
        .add_arg(right.clone())
        .run_with_input(PipelineData::Empty)
        .and_then(|data| data.into_value(span))
        .map(|val| {
            if val.is_true() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
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
