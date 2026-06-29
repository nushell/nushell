#[macro_export]
macro_rules! record {
    // The macro only compiles if the number of columns equals the number of values,
    // so it's safe to call `unwrap` below.
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Record::from_raw_cols_vals(
            ::std::vec![$($col.into(),)+],
            ::std::vec![$($val,)+],
            $crate::Span::unknown(),
            $crate::Span::unknown(),
        ).unwrap()
    };
    {} => {
        $crate::Record::new()
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
    {$($col:expr => $val:expr),+ $(,)?} => {
        $crate::Value::test_record($crate::record! {
            $($col => $crate::IntoValue::into_value($val, $crate::Span::test_data())),+
        })
    };
    {} => {
        $crate::Value::test_record($crate::record! {})
    };
}

#[doc(hidden)]
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
/// ```rust
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
