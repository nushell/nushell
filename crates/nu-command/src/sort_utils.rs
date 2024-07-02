use alphanumeric_sort::compare_str;
use nu_engine::ClosureEval;
use nu_protocol::{
    ast::CellPath,
    engine::{Closure, EngineState, Stack},
    PipelineData, ShellError, Span, Value,
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

/// Sort a value. This only makes sense for lists and list-like things,
/// so for everything else we just return the value as-is.
/// CustomValues are converted to their base value and then sorted.
pub fn sort_value(
    val: &Value,
    comparators: Vec<Comparator>,
    ascending: bool,
    insensitive: bool,
    natural: bool,
) -> Result<Value, ShellError> {
    let span = val.span();
    match val {
        Value::List { vals, .. } => {
            let mut vals = vals.clone();
            sort(&mut vals, comparators, span, insensitive, natural)?;

            if !ascending {
                vals.reverse();
            }

            Ok(Value::list(vals, span))
        }
        Value::Custom { val, .. } => {
            let base_val = val.to_base_value(span)?;
            sort_value(&base_val, comparators, ascending, insensitive, natural)
        }
        _ => Ok(val.to_owned()),
    }
}

/// Sort a value in-place. This is more efficient than sort_value() because it
/// avoids cloning, but it does not work for CustomValues; they are returned as-is.
pub fn sort_value_in_place(
    val: &mut Value,
    comparators: Vec<Comparator>,
    ascending: bool,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    let span = val.span();
    if let Value::List { vals, .. } = val {
        sort(vals, comparators, span, insensitive, natural)?;
        if !ascending {
            vals.reverse();
        }
    }
    Ok(())
}

pub fn sort(
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
        compare(
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

pub fn compare(
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

pub fn compare_cell_path(
    left: &Value,
    right: &Value,
    cell_path: &CellPath,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    let left = left.clone().follow_cell_path(&cell_path.members, false)?;
    let right = right.clone().follow_cell_path(&cell_path.members, false)?;

    if insensitive || natural {
        let mut left_str = left.coerce_into_string()?;
        let mut right_str = right.coerce_into_string()?;
        if insensitive {
            left_str = left_str.to_folded_case();
            right_str = right_str.to_folded_case();
        }

        if natural {
            Ok(compare_str(left_str, right_str))
        } else {
            Ok(left_str.partial_cmp(&right_str).unwrap_or(Ordering::Equal))
        }
    } else {
        Ok(left.partial_cmp(&right).unwrap_or(Ordering::Equal))
    }
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
