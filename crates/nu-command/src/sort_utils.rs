use nu_engine::ClosureEval;
use nu_protocol::shell_error::generic::GenericError;
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
        return Err(ShellError::Generic(GenericError::new(
            "expected name",
            "requires a cell path or closure to sort data",
            head_span,
        )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{ast::PathMember, casing::Casing, record};

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
        let mut values: Vec<Value> =
            itertools::intersperse(Value::test_values(), Value::test_nothing()).collect();

        let nulls = values
            .iter()
            .filter(|item| item == &&Value::test_nothing())
            .count();

        assert!(sort(&mut values, false, false).is_ok());

        // check if the last `nulls` values of the sorted list are indeed null
        assert_eq!(&values[(nulls - 1)..], vec![Value::test_nothing(); nulls])
    }

    #[test]
    fn test_sort_natural_basic() {
        let mut list = vec![
            Value::test_string("foo99"),
            Value::test_string("foo9"),
            Value::test_string("foo1"),
            Value::test_string("foo100"),
            Value::test_string("foo10"),
            Value::test_string("1"),
            Value::test_string("10"),
            Value::test_string("100"),
            Value::test_string("9"),
            Value::test_string("99"),
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
                Value::test_string("foo1"),
                Value::test_string("foo10"),
                Value::test_string("foo100"),
                Value::test_string("foo9"),
                Value::test_string("foo99"),
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
                Value::test_string("foo1"),
                Value::test_string("foo9"),
                Value::test_string("foo10"),
                Value::test_string("foo99"),
                Value::test_string("foo100"),
            ]
        );
    }

    #[test]
    fn test_sort_natural_mixed_types() {
        let mut list = vec![
            Value::test_string("1"),
            Value::test_int(99),
            Value::test_int(1),
            Value::test_float(1000.0),
            Value::test_int(9),
            Value::test_string("9"),
            Value::test_int(100),
            Value::test_string("99"),
            Value::test_float(2.0),
            Value::test_string("100"),
            Value::test_int(10),
            Value::test_string("10"),
        ];

        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(1),
                Value::test_float(2.0),
                Value::test_int(9),
                Value::test_int(10),
                Value::test_int(99),
                Value::test_int(100),
                Value::test_float(1000.0),
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
                Value::test_float(2.0),
                Value::test_int(9),
                Value::test_string("9"),
                Value::test_int(10),
                Value::test_string("10"),
                Value::test_int(99),
                Value::test_string("99"),
                Value::test_int(100),
                Value::test_string("100"),
                Value::test_float(1000.0),
            ]
        );
    }

    #[test]
    fn test_sort_natural_no_numeric_values() {
        // If list contains no numeric strings, it should be sorted the
        // same with or without natural sorting
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
                Value::test_bool(false),
                Value::test_bool(false),
                Value::test_bool(true),
                Value::test_bool(true),
                Value::test_int(1),
                Value::test_int(3),
                Value::test_int(7),
                Value::test_int(10),
                Value::test_string("alfa"),
                Value::test_string("golf"),
                Value::test_string("tango"),
                Value::test_string("uniform")
            ]
        );

        // Only ints, floats, and numeric strings should be intermixed
        // While binary primitives and datetimes can be coerced into strings, it doesn't make sense to sort them with numbers
        // Binary primitives can hold multiple values, not just one, so shouldn't be compared to single values
        // Datetimes don't have a single obvious numeric representation, and if we chose one it would be ambiguous to the user

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
            Value::test_string("foobar"),
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
                Value::test_string("foobar"),
                // the ordering of date and binary here may change if the PartialOrd order is changed,
                // but they should not be intermixed with the above
                Value::test_date(year_three.into()),
                Value::test_date(chrono::DateTime::UNIX_EPOCH.into()),
                Value::test_binary([3]),
                Value::test_binary([52]),
            ]
        );
    }

    #[test]
    fn test_sort_insensitive() {
        // Test permutations between insensitive and natural
        // Ensure that strings with equal insensitive orderings
        // are sorted stably. (FOO then foo, bar then BAR)
        let source = vec![
            Value::test_string("FOO"),
            Value::test_string("foo"),
            Value::test_int(100),
            Value::test_string("9"),
            Value::test_string("bar"),
            Value::test_int(10),
            Value::test_string("baz"),
            Value::test_string("BAR"),
        ];
        let mut list;

        // sensitive + non-natural
        list = source.clone();
        assert!(sort(&mut list, false, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(10),
                Value::test_int(100),
                Value::test_string("9"),
                Value::test_string("BAR"),
                Value::test_string("FOO"),
                Value::test_string("bar"),
                Value::test_string("baz"),
                Value::test_string("foo"),
            ]
        );

        // sensitive + natural
        list = source.clone();
        assert!(sort(&mut list, false, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_string("9"),
                Value::test_int(10),
                Value::test_int(100),
                Value::test_string("BAR"),
                Value::test_string("FOO"),
                Value::test_string("bar"),
                Value::test_string("baz"),
                Value::test_string("foo"),
            ]
        );

        // insensitive + non-natural
        list = source.clone();
        assert!(sort(&mut list, true, false).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_int(10),
                Value::test_int(100),
                Value::test_string("9"),
                Value::test_string("bar"),
                Value::test_string("BAR"),
                Value::test_string("baz"),
                Value::test_string("FOO"),
                Value::test_string("foo"),
            ]
        );

        // insensitive + natural
        list = source.clone();
        assert!(sort(&mut list, true, true).is_ok());
        assert_eq!(
            list,
            vec![
                Value::test_string("9"),
                Value::test_int(10),
                Value::test_int(100),
                Value::test_string("bar"),
                Value::test_string("BAR"),
                Value::test_string("baz"),
                Value::test_string("FOO"),
                Value::test_string("foo"),
            ]
        );
    }

    // Helper function to assert that two records are equal
    // with their key-value pairs in the same order
    fn assert_record_eq(a: Record, b: Record) {
        assert_eq!(
            a.into_iter().collect::<Vec<_>>(),
            b.into_iter().collect::<Vec<_>>(),
        )
    }

    #[test]
    fn test_sort_record_keys() {
        // Basic record sort test
        let record = record! {
            "golf" => Value::test_string("bar"),
            "alfa" => Value::test_string("foo"),
            "echo" => Value::test_int(123),
        };

        let sorted = sort_record(record, false, false, false, false).unwrap();
        assert_record_eq(
            sorted,
            record! {
                "alfa" => Value::test_string("foo"),
                "echo" => Value::test_int(123),
                "golf" => Value::test_string("bar"),
            },
        );
    }

    #[test]
    fn test_sort_record_values() {
        // This test is to prevent a regression where integers and strings would be
        // intermixed non-naturally when sorting a record by value without the natural flag:
        //
        // This record would previously be incorrectly sorted like this:
        // ╭─────────┬─────╮
        // │ alfa    │ 1   │
        // │ charlie │ 1   │
        // │ india   │ 10  │
        // │ juliett │ 10  │
        // │ foxtrot │ 100 │
        // │ hotel   │ 100 │
        // │ delta   │ 9   │
        // │ echo    │ 9   │
        // │ bravo   │ 99  │
        // │ golf    │ 99  │
        // ╰─────────┴─────╯

        let record = record! {
            "alfa" => Value::test_string("1"),
            "bravo" => Value::test_int(99),
            "charlie" => Value::test_int(1),
            "delta" => Value::test_int(9),
            "echo" => Value::test_string("9"),
            "foxtrot" => Value::test_int(100),
            "golf" => Value::test_string("99"),
            "hotel" => Value::test_string("100"),
            "india" => Value::test_int(10),
            "juliett" => Value::test_string("10"),
        };

        // non-natural sort
        let sorted = sort_record(record.clone(), true, false, false, false).unwrap();
        assert_record_eq(
            sorted,
            record! {
                "charlie" => Value::test_int(1),
                "delta" => Value::test_int(9),
                "india" => Value::test_int(10),
                "bravo" => Value::test_int(99),
                "foxtrot" => Value::test_int(100),
                "alfa" => Value::test_string("1"),
                "juliett" => Value::test_string("10"),
                "hotel" => Value::test_string("100"),
                "echo" => Value::test_string("9"),
                "golf" => Value::test_string("99"),
            },
        );

        // natural sort
        let sorted = sort_record(record.clone(), true, false, false, true).unwrap();
        assert_record_eq(
            sorted,
            record! {
                "alfa" => Value::test_string("1"),
                "charlie" => Value::test_int(1),
                "delta" => Value::test_int(9),
                "echo" => Value::test_string("9"),
                "india" => Value::test_int(10),
                "juliett" => Value::test_string("10"),
                "bravo" => Value::test_int(99),
                "golf" => Value::test_string("99"),
                "foxtrot" => Value::test_int(100),
                "hotel" => Value::test_string("100"),
            },
        );
    }

    #[test]
    fn test_sort_equivalent() {
        // Ensure that sort, sort_by, and record sort have equivalent sorting logic
        let phonetic = vec![
            "alfa", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india",
            "juliett", "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo",
            "sierra", "tango", "uniform", "victor", "whiskey", "xray", "yankee", "zulu",
        ];

        // filter out errors, since we can't sort_by on those
        let mut values: Vec<Value> = Value::test_values()
            .into_iter()
            .filter(|val| !matches!(val, Value::Error { .. }))
            .collect();

        // reverse sort test values
        values.sort_by(|a, b| b.partial_cmp(a).unwrap());

        let mut list = values.clone();
        let mut table: Vec<Value> = values
            .clone()
            .into_iter()
            .map(|val| Value::test_record(record! { "value" => val }))
            .collect();
        let record = Record::from_iter(phonetic.into_iter().map(str::to_string).zip(values));

        let comparator = Comparator::CellPath(CellPath {
            members: vec![PathMember::String {
                val: "value".to_string(),
                span: Span::test_data(),
                optional: false,
                casing: Casing::Sensitive,
            }],
        });

        assert!(sort(&mut list, false, false).is_ok());
        assert!(
            sort_by(
                &mut table,
                vec![comparator],
                Span::test_data(),
                false,
                false
            )
            .is_ok()
        );

        let record_sorted = sort_record(record.clone(), true, false, false, false).unwrap();
        let record_vals: Vec<Value> = record_sorted.into_iter().map(|pair| pair.1).collect();

        let table_vals: Vec<Value> = table
            .clone()
            .into_iter()
            .map(|record| record.into_record().unwrap().remove("value").unwrap())
            .collect();

        assert_eq!(list, record_vals);
        assert_eq!(record_vals, table_vals);
        // list == table_vals by transitive property
    }
}
