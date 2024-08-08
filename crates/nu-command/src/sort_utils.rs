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
        Ok(left.partial_cmp(right).unwrap_or(Ordering::Equal))
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
    use nu_protocol::Value;

    #[test]
    fn test_sort_basic() {
        let mut list = vec![
            Value::test_string("foo"),
            Value::test_int(2),
            Value::test_int(3),
            Value::test_string("bar"),
            Value::test_int(1),
            Value::test_string("baz"),
        ];

        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
                Value::test_string("bar"),
                Value::test_string("baz"),
                Value::test_string("foo")
            ]
        );
    }

    #[test]
    fn test_sort_nothing() {
        // Nothing values should always be sorted to the end of any list
        let mut list = vec![
            Value::test_int(1),
            Value::test_nothing(),
            Value::test_int(2),
            Value::test_string("foo"),
            Value::test_nothing(),
            Value::test_string("bar"),
        ];

        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_string("bar"),
                Value::test_string("foo"),
                Value::test_nothing(),
                Value::test_nothing()
            ]
        );

        // Ensure that nothing values are sorted after *all* types,
        // even types which may follow `Nothing` in the PartialOrd order

        // unstable_name_collision
        // can be switched to std intersperse when stabilized
        let mut values: Vec<_> =
            itertools::intersperse(Value::test_values().into_iter(), Value::test_nothing())
                .collect();

        let nulls = values
            .iter()
            .filter(|item| item == &&Value::test_nothing())
            .count();

        assert!(sort(&mut values, false, false).is_ok());

        // check if the last `nulls` values of the sorted list are indeed null
        assert_eq!(&values[..nulls], vec![Value::test_nothing(); nulls])
    }

    #[test]
    fn test_sort_natural_basic() {
        let mut list = vec![
            Value::test_string("99"),
            Value::test_string("9"),
            Value::test_string("1"),
            Value::test_string("100"),
            Value::test_string("10"),
        ];

        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_string("1"),
                Value::test_string("10"),
                Value::test_string("100"),
                Value::test_string("9"),
                Value::test_string("99"),
            ]
        );

        assert!(sort(&mut list, false, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_string("1"),
                Value::test_string("9"),
                Value::test_string("10"),
                Value::test_string("99"),
                Value::test_string("100"),
            ]
        );
    }

    #[test]
    fn test_sort_natural_mixed_types() {
        let mut list = vec![
            Value::test_string("1"),
            Value::test_int(99),
            Value::test_int(1),
            Value::test_int(9),
            Value::test_string("9"),
            Value::test_int(100),
            Value::test_string("99"),
            Value::test_string("100"),
            Value::test_int(10),
            Value::test_string("10"),
        ];

        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_int(9),
                Value::test_int(10),
                Value::test_int(99),
                Value::test_int(100),
                Value::test_string("1"),
                Value::test_string("10"),
                Value::test_string("100"),
                Value::test_string("9"),
                Value::test_string("99")
            ]
        );

        assert!(sort(&mut list, false, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_string("1"),
                Value::test_int(9),
                Value::test_string("9"),
                Value::test_int(10),
                Value::test_string("10"),
                Value::test_int(99),
                Value::test_string("99"),
                Value::test_int(100),
                Value::test_string("100"),
            ]
        );
    }

    #[test]
    fn test_sort_natural_no_numeric_values() {
        // If list contains no numeric values (numeric strings, ints, floats),
        // it should be sorted the same with or without natural sorting
        let mut normal = vec![
            Value::test_string("golf"),
            Value::test_bool(false),
            Value::test_string("alfa"),
            Value::test_string("echo"),
            Value::test_int(7),
            Value::test_int(10),
            Value::test_bool(true),
            Value::test_string("uniform"),
            Value::test_int(3),
            Value::test_string("tango"),
        ];
        let mut natural = normal.clone();

        assert!(sort(&mut normal, false, false).is_ok());
        assert!(sort(&mut natural, false, true).is_ok());
        assert_eq!(normal, natural);
    }

    #[test]
    fn test_sort_natural_type_order() {
        // This test is to prevent regression to a previous natural sort behavior
        // where values of different types would be intermixed.
        // Only numeric values (ints, floats, and numeric strings) should be intermixed
        //
        // This list would previously be incorrectly sorted like this:
        // ╭────┬─────────╮
        // │  0 │       1 │
        // │  1 │ golf    │
        // │  2 │ false   │
        // │  3 │       7 │
        // │  4 │      10 │
        // │  5 │ alfa    │
        // │  6 │ true    │
        // │  7 │ uniform │
        // │  8 │ true    │
        // │  9 │       3 │
        // │ 10 │ false   │
        // │ 11 │ tango   │
        // ╰────┴─────────╯

        let mut list = vec![
            Value::test_string("golf"),
            Value::test_int(1),
            Value::test_bool(false),
            Value::test_string("alfa"),
            Value::test_int(7),
            Value::test_int(10),
            Value::test_bool(true),
            Value::test_string("uniform"),
            Value::test_bool(true),
            Value::test_int(3),
            Value::test_bool(false),
            Value::test_string("tango"),
        ];

        assert!(sort(&mut list, false, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_int(3),
                Value::test_int(7),
                Value::test_int(10),
                Value::test_bool(false),
                Value::test_bool(false),
                Value::test_bool(true),
                Value::test_bool(true),
                Value::test_string("alfa"),
                Value::test_string("golf"),
                Value::test_string("tango"),
                Value::test_string("uniform")
            ]
        );

        // Only ints, floats, and numeric strings should be intermixed
        // While binary primitives and datetimes can be coerced into strings, it doesn't make sense to sort them with numbers
        // Binary primitives can hold multiple values, not just one, so shouldn't be compared to single values
        // Datetimes don't have a single obvious numeric representation, and if we chose one it would be ambigious to the user

        let year_three = chrono::NaiveDate::from_ymd_opt(3, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let mut list = vec![
            Value::test_int(10),
            Value::test_float(6.0),
            Value::test_int(1),
            Value::test_binary([3]),
            Value::test_string("2"),
            Value::test_date(year_three.into()),
            Value::test_int(4),
            Value::test_binary([52]),
            Value::test_float(9.0),
            Value::test_string("5"),
            Value::test_date(chrono::DateTime::UNIX_EPOCH.into()),
            Value::test_int(7),
            Value::test_string("8"),
            Value::test_float(3.0),
        ];
        assert!(sort(&mut list, false, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_string("2"),
                Value::test_float(3.0),
                Value::test_int(4),
                Value::test_string("5"),
                Value::test_float(6.0),
                Value::test_int(7),
                Value::test_string("8"),
                Value::test_float(9.0),
                Value::test_int(10),
                // the ordering of date and binary here may change if the PartialOrd order is changed,
                // but they should not be intermixed with the above
                Value::test_binary([3]),
                Value::test_binary([52]),
                Value::test_date(year_three.into()),
                Value::test_date(chrono::DateTime::UNIX_EPOCH.into()),
            ]
        );
    }
}
