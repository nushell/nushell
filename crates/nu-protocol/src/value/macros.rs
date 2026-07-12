#[macro_export]
macro_rules! record {
    {$($col:expr => $val:expr),* $(,)?} => {
        $crate::Record::from_iter(::std::vec![ $(
            (::std::string::String::from($col), $val)
        ),* ])
    };
}

/// Helper for constructing [Value::Record] instances for use in tests and
/// [Example](crate::Example)s
/// ```
/// # use nu_protocol::{Value, test_record, record};
/// let test = test_record! {
///     "a" => "foo",
///     "b" => 42,
///     "c" => [1, 2, 3],
/// };
///
/// let expected = Value::test_record(record! {
///     "a" => Value::test_string("foo"),
///     "b" => Value::test_int(42),
///     "c" => Value::test_list(vec![
///         Value::test_int(1),
///         Value::test_int(2),
///         Value::test_int(3),
///     ]),
/// });
///
/// assert_eq!(test, expected);
/// ```
#[macro_export]
macro_rules! test_record {
    {$($col:expr => $val:expr),* $(,)?} => {
        $crate::Value::test_record($crate::record! { $(
            $col => $crate::IntoValue::into_value($val, $crate::Span::test_data())
        ),* })
    };
}

#[doc(hidden)]
#[allow(unused)]
pub const fn count_helper<const N: usize>(_: [(); N]) -> usize {
    N
}

/// Helper for constructing table (list of records) values for use in tests and
/// [Example](crate::Example)s
/// ```
/// # use nu_protocol::{Value, test_table, test_record, record};
/// let test = test_table![
///     ["a", "b", "c"];
///     [1, 2, 3],
///     [4, 5, 6],
/// ];
///
/// let expected = Value::test_list(vec![
///     test_record! {"a" => 1, "b" => 2, "c" => 3},
///     test_record! {"a" => 4, "b" => 5, "c" => 6},
/// ]);
///
/// assert_eq!(test, expected);
/// ```
#[macro_export]
macro_rules! test_table {
    (@replace_expr $_t:tt $sub:expr) => { $sub };
    (@count_tts $($smth:tt)*) => {
        $crate::macros::count_helper([$($crate::test_table!(@replace_expr $smth ())),*])
    };
    [[$($col:expr),+ $(,)?]; $([$($val:expr),+ $(,)?]),+ $(,)?] => {{
        const COLUMNS: usize = $crate::test_table!(@count_tts $($col)+);
        let columns: ::std::vec::Vec<::std::string::String> = ::std::vec![$($col.into()),+];
        let rows = vec![ $(
            {
                const ROW_ITEMS: usize = $crate::test_table!(@count_tts $($val)+);
                const _: () = assert!(ROW_ITEMS == COLUMNS) ;
                $crate::Value::test_record($crate::Record::from_raw_cols_vals(
                    columns.clone(),
                    ::std::vec![ $(
                        $crate::IntoValue::into_value($val, $crate::Span::test_data())
                    ),+ ],
                    $crate::Span::test_data(),
                    $crate::Span::test_data(),
                ).expect("Number of columns and rows should be equal"))
            }
        ),+ ];
        $crate::Value::test_list(rows)
    }};
}

/// Helper macro for constructing [`Value::List`] instances for use in tests and
/// [Examples](crate::Example)s.
///
/// ```
/// # use nu_protocol::*;
/// #
/// let test = test_list![
///     "abc",
///     42,
///     true,
/// ];
///
/// let expected = Value::test_list(vec![
///     Value::test_string("abc"),
///     Value::test_int(42),
///     Value::test_bool(true),
/// ]);
///
/// assert_eq!(test, expected);
/// ```
#[macro_export]
macro_rules! test_list {
    [$($entry:expr),* $(,)?] => {
        $crate::Value::test_list(::std::vec![
            $($crate::IntoValue::into_value($entry, $crate::Span::test_data())),*
        ])
    };
}

/// Helper macro for constructing [`Value`] instances for use in tests and
/// [Examples](crate::Example)s.
///
/// Can be used to create simple scalar values with anything implementing
/// [IntoValue](crate::IntoValue):
/// ```
/// # use nu_protocol::*;
/// assert_eq!(test_value!(42),   Value::test_int(42));
/// assert_eq!(test_value!(true), Value::test_bool(true));
/// assert_eq!(test_value!(()),   Value::test_nothing());
/// ```
///
/// Can be used in place of [`test_list!`]:
/// ```
/// # use nu_protocol::*;
/// let test =   test_value!(["abc", 42, true]);
/// let expected = test_list!["abc", 42, true];
/// assert_eq!(test, expected);
/// ```
///
/// Can be used in place of [`test_record!`], with some differences:
/// - instead of fat arrows (`=>`), colons are used (`:`).
/// - keys can be bare identifiers in addition to string literals and variables.
///   (to use the value of an existing variable, wrap it in parentheses)
/// ```
/// # use nu_protocol::*;
/// let key_in_var = "foo";
/// let test = test_value!({
///     a: 1,
///     "b": 2,
///     (key_in_var): "bar",
/// });
/// let expected = test_record! {
///     "a" => 1,
///     "b" => 2,
///     "foo" => "bar",
/// };
/// assert_eq!(test, expected);
/// ```
///
/// The most important feature of [`test_value!`] is that it works recursively for all values.
/// That makes it very powerful for constructing complex and nested values:
/// ```
/// # use nu_protocol::*;
/// let test = test_value!({
///     a: 1,
///     b: {
///         c: 2,
///         d: ["e", "f", {g: 3}],
///     },
/// });
/// let expected = test_record! {
///     "a" => 1,
///     "b" => test_record! {
///         "c" => 2,
///         "d" => test_list! ["e", "f", test_record! { "g" => 3 } ],
///     },
/// };
/// assert_eq!(test, expected);
/// ```
#[macro_export]
macro_rules! test_value {
    (@recur, [$($item:tt),* $(,)?]) => {
        $crate::test_list![$(
            $crate::test_value!(@recur, $item)
        ),*]
    };
    (@recur, {$($col:tt : $val:tt),* $(,)?}) => {
        $crate::test_record! { $(
            $crate::test_value!(@col, $col) => $crate::test_value!(@recur, $val)
        ),* }
    };
    (@recur, $val:expr) => { $val };

    (@col, $col:ident) => { stringify!($col) };
    (@col, $col:expr) => { $col };

    // top level calls
    ([$($item:tt),* $(,)?]) => { $crate::test_value!(@recur, [$($item),*]) };
    ({$($col:tt : $val:tt),* $(,)?}) => { $crate::test_value!(@recur, {$($col : $val),*}) };
    ($val:expr) => { $crate::IntoValue::into_value($val, $crate::Span::test_data()) };
}

#[cfg(test)]
mod test_value_macro_tests {
    use pretty_assertions::assert_eq;

    use crate::{IntoValue, Span};

    #[test]
    fn ident_record_columns() {
        let foo_col = "foo_val";
        let x = test_value!({
            a: 2,
            b: 3,
            foo_col: foo_col,
            (foo_col): foo_col,
        });

        let expected = test_record! {
            "a" => 2,
            "b" => 3,
            "foo_col" => "foo_val",
            "foo_val" => "foo_val",
        };

        assert_eq!(x, expected);
    }

    #[test]
    fn simple_values() {
        let x = test_value!(10);
        let expected = 10.into_value(Span::test_data());

        assert_eq!(x, expected);

        let x = test_value!(true);
        let expected = true.into_value(Span::test_data());

        assert_eq!(x, expected);

        let x = test_value!(());
        let expected = ().into_value(Span::test_data());

        assert_eq!(x, expected);
    }

    #[test]
    fn simple_record() {
        let x = test_value!({
            "a": 1,
            "b": 2,
            "c": 3,
        });

        let expected = test_record! {
            "a" => 1,
            "b" => 2,
            "c" => 3,
        };

        assert_eq!(x, expected);
    }

    #[test]
    fn simple_list() {
        let x = test_value!(["abc", 42, true,]);

        let expected = test_list!["abc", 42, true,];

        assert_eq!(x, expected);
    }

    #[test]
    fn nested_records() {
        let x = test_value!({
            "a": 1,
            "b": 2,
            "c": {
                "d": 4,
                "e": 5,
                "f": {
                    "g": 7,
                    "h": 8,
                }
            },
        });

        let expected = test_record! {
            "a" => 1,
            "b" => 2,
            "c" => test_record! {
                "d" => 4,
                "e" => 5,
                "f" => test_record! {
                    "g" => 7,
                    "h" => 8,
                }
            },
        };

        assert_eq!(x, expected);
    }

    #[test]
    fn nested_lists() {
        let x = test_value!(["a", "b", ["c", "d", ["e", "f",],],]);

        let expected = test_list!["a", "b", test_list!["c", "d", test_list!["e", "f",],],];

        assert_eq!(x, expected);
    }

    #[test]
    fn complex_value() {
        let x = test_value!({
            "a": 1,
            "b": {
                "b_a": 3,
                "b_b": 4,
            },
            "c": [1, "two", ()],
            "d": [
                {"foo": 1, "bar": 10},
                {"foo": 2, "bar": 20},
                {"foo": 3, "bar": 30},
            ],
        });

        let expected = test_record! {
            "a" => 1,
            "b" => test_record! {
                "b_a" => 3,
                "b_b" => 4,
            },
            "c" => test_list![1, "two", ()],
            "d" => test_list![
                test_record! {"foo" => 1, "bar" => 10},
                test_record! {"foo" => 2, "bar" => 20},
                test_record! {"foo" => 3, "bar" => 30},
            ],
        };

        assert_eq!(x, expected)
    }

    #[test]
    fn complex_value_with_ident_keys() {
        let x = test_value!({
            a: 1,
            b: {
                b_a: 3,
                b_b: 4,
            },
            c: [1, "two", ()],
            d: [
                {foo: 1, bar: 10},
                {foo: 2, bar: 20},
                {foo: 3, bar: 30},
            ],
        });

        let expected = test_record! {
            "a" => 1,
            "b" => test_record! {
                "b_a" => 3,
                "b_b" => 4,
            },
            "c" => test_list![1, "two", ()],
            "d" => test_list![
                test_record! {"foo" => 1, "bar" => 10},
                test_record! {"foo" => 2, "bar" => 20},
                test_record! {"foo" => 3, "bar" => 30},
            ],
        };

        assert_eq!(x, expected)
    }
}
