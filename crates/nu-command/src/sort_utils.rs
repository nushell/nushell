use nu_engine::ClosureEval;
use nu_protocol::{PipelineData, Record, ShellError, Span, Value, ast::CellPath};
use nu_utils::IgnoreCaseExt;
use std::cmp::Ordering;

/// A specification of sort order for `sort_by`.
///
/// A closure comparator allows the user to return custom ordering to sort by.
/// A cell path comparator uses the value referred to by the cell path as the sorting key.
pub enum Comparator {
    KeyClosure(ClosureEval),
    CustomClosure(ClosureEval),
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
    // allow the comparator function to indicate error
    // by mutating this option captured by the closure,
    // since sort_by closure must be infallible
    let mut compare_err: Option<ShellError> = None;

    vec.sort_by(|a, b| {
        // we've already hit an error, bail out now
        if compare_err.is_some() {
            return Ordering::Equal;
        }

        compare_values(a, b, insensitive, natural).unwrap_or_else(|err| {
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
    mut comparators: Vec<Comparator>,
    head_span: Span,
    insensitive: bool,
    natural: bool,
) -> Result<(), ShellError> {
    if comparators.is_empty() {
        return Err(ShellError::GenericError {
            error: "expected name".into(),
            msg: "requires a cell path or closure to sort data".into(),
            span: Some(head_span),
            help: None,
            inner: vec![],
        });
    }

    // allow the comparator function to indicate error
    // by mutating this option captured by the closure,
    // since sort_by closure must be infallible
    let mut compare_err: Option<ShellError> = None;

    vec.sort_by(|a, b| {
        compare_by(
            a,
            b,
            &mut comparators,
            head_span,
            insensitive,
            natural,
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

    if sort_by_value {
        input_pairs.sort_by(|a, b| {
            // we've already hit an error, bail out now
            if compare_err.is_some() {
                return Ordering::Equal;
            }

            compare_values(&a.1, &b.1, insensitive, natural).unwrap_or_else(|err| {
                compare_err.get_or_insert(err);
                Ordering::Equal
            })
        });
    } else {
        input_pairs.sort_by(|a, b| compare_strings(&a.0, &b.0, insensitive, natural));
    };

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
    comparators: &mut [Comparator],
    span: Span,
    insensitive: bool,
    natural: bool,
    error: &mut Option<ShellError>,
) -> Ordering {
    // we've already hit an error, bail out now
    if error.is_some() {
        return Ordering::Equal;
    }
    for cmp in comparators.iter_mut() {
        let result = match cmp {
            Comparator::CellPath(cell_path) => {
                compare_cell_path(left, right, cell_path, insensitive, natural)
            }
            Comparator::KeyClosure(closure) => {
                compare_key_closure(left, right, closure, span, insensitive, natural)
            }
            Comparator::CustomClosure(closure) => {
                compare_custom_closure(left, right, closure, span)
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

/// Determines whether a value should be sorted as a string
///
/// If we're natural sorting, we want to sort strings, integers, and floats alphanumerically, so we should string sort.
/// Otherwise, we only want to string sort if both values are strings or globs (to enable case insensitive comparison)
fn should_sort_as_string(val: &Value, natural: bool) -> bool {
    matches!(
        (val, natural),
        (&Value::String { .. }, _)
            | (&Value::Glob { .. }, _)
            | (&Value::Int { .. }, true)
            | (&Value::Float { .. }, true)
    )
}

/// Simple wrapper around `should_sort_as_string` to determine if two values
/// should be compared as strings.
fn should_string_compare(left: &Value, right: &Value, natural: bool) -> bool {
    should_sort_as_string(left, natural) && should_sort_as_string(right, natural)
}

pub fn compare_values(
    left: &Value,
    right: &Value,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    if should_string_compare(left, right, natural) {
        Ok(compare_strings(
            &left.coerce_str()?,
            &right.coerce_str()?,
            insensitive,
            natural,
        ))
    } else {
        Ok(left.partial_cmp(right).unwrap_or(Ordering::Equal))
    }
}

pub fn compare_strings(left: &str, right: &str, insensitive: bool, natural: bool) -> Ordering {
    fn compare_inner<T>(left: T, right: T, natural: bool) -> Ordering
    where
        T: AsRef<str> + Ord,
    {
        if natural {
            alphanumeric_sort::compare_str(left, right)
        } else {
            left.cmp(&right)
        }
    }

    // only allocate a String if necessary for case folding
    if insensitive {
        compare_inner(left.to_folded_case(), right.to_folded_case(), natural)
    } else {
        compare_inner(left, right, natural)
    }
}

pub fn compare_cell_path(
    left: &Value,
    right: &Value,
    cell_path: &CellPath,
    insensitive: bool,
    natural: bool,
) -> Result<Ordering, ShellError> {
    let left = left.follow_cell_path(&cell_path.members)?;
    let right = right.follow_cell_path(&cell_path.members)?;
    compare_values(&left, &right, insensitive, natural)
}

pub fn compare_key_closure(
    left: &Value,
    right: &Value,
    closure_eval: &mut ClosureEval,
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
    closure_eval: &mut ClosureEval,
    span: Span,
) -> Result<Ordering, ShellError> {
    closure_eval
        .add_arg(left.clone())
        .add_arg(right.clone())
        .run_with_input(PipelineData::value(
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
