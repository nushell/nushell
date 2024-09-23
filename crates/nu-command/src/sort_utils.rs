use nu_engine::ClosureEval;
use nu_protocol::{
    ast::CellPath,
    engine::{Closure, EngineState, Stack},
    PipelineData, Record, ShellError, Span, Value,
};
use nu_utils::IgnoreCaseExt;
use std::cmp::Ordering;

/// A specification of sort order for `sort_by`.
///
/// A closure comparator allows the user to return custom ordering to sort by.
/// A cell path comparator uses the value referred to by the cell path as the sorting key.
pub enum Comparator {
    KeyClosure(Closure, EngineState, Stack),
    CustomClosure(Closure, EngineState, Stack),
    CellPath(CellPath),
}

/// Sort a slice of `Value`s.
///
/// Sort has the following invariants, in order of precedence:
/// - Null values (Nothing type) are always sorted to the end.
/// - For natural sort, numeric values (numeric strings, ints, and floats) appear first, sorted by numeric value
/// - Values appear by order of `Value`'s `PartialOrd`.
/// - Sorting for values with equal ordering is stable.
///
/// Generally, values of different types are ordered by order of appearance in the `Value` enum.
/// However, this is not always the case. For example, ints and floats will be grouped together since
/// `Value`'s `PartialOrd` defines a non-decreasing ordering between non-decreasing integers and floats.
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

/// Sort a slice of `Value`s by criteria specified by one or multiple `Comparator`s.
pub fn sort_by(
    vec: &mut [Value],
    comparators: Vec<Comparator>,
    head_span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    if comparators.is_empty() {
        // This uses the same format as the 'requires a column name' error in split_by.rs
        return Err(ShellError::GenericError {
            error: "expected name".into(),
            msg: "requires a cell path or closure to sort data".into(),
            span: Some(head_span),
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
            head_span,
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

/// Sort a record's key-value pairs.
///
/// Can sort by key or by value.
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

    if let Some(err) = compare_err {
        return Err(err);
    }

    if reverse {
        input_pairs.reverse()
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
            Comparator::KeyClosure(closure, engine_state, stack) => {
                let closure_eval = ClosureEval::new(engine_state, stack, closure.clone());
                compare_key_closure(left, right, closure_eval, span, insensitive, natural)
            }
            Comparator::CustomClosure(closure, engine_state, stack) => {
                let closure_eval = ClosureEval::new(engine_state, stack, closure.clone());
                compare_custom_closure(left, right, closure_eval, span)
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
        Ok(left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }
}

pub fn compare_strings(
    left: &String,
    right: &String,
    insensitive: bool,
    natural: bool,
) -> Ordering {
    // only allocate new String if necessary for case folding,
    // so callers don't need to pass an owned String
    let (left_str, right_str) = if insensitive {
        (&left.to_folded_case(), &right.to_folded_case())
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

pub fn compare_key_closure(
    left: &Value,
    right: &Value,
    mut closure_eval: ClosureEval,
    span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    let left_key = closure_eval
        .run_with_value(left.clone())?
        .into_value(span)?;
    let right_key = closure_eval
        .run_with_value(right.clone())?
        .into_value(span)?;
    compare_values(&left_key, &right_key, insensitive, natural)
}

pub fn compare_custom_closure(
    left: &Value,
    right: &Value,
    mut closure_eval: ClosureEval,
    span: Span,
) -> Result<Ordering, ShellError> {
    closure_eval
        .add_arg(left.clone())
        .add_arg(right.clone())
        .run_with_input(PipelineData::Value(
            Value::list(vec![left.clone(), right.clone()], span),
            None,
        ))
        .and_then(|data| data.into_value(span))
        .map(|val| {
            if val.is_true() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        })
}
