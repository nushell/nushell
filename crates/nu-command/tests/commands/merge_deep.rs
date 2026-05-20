use nu_test_support::prelude::*;
use rstest::rstest;

#[derive(Debug, IntoValue)]
enum Strategy {
    Table,
    Append,
    Prepend,
    Overwrite,
}

mod table {
    pub const LEFT: &str = "{ inner: [
        {a: 1},
        {b: 2},
    ]}";
    pub const RIGHT: &str = "{ inner: [
        {c: 3},
    ]}";

    pub const TABLE: &str = "{ inner: [
        {a: 1      c: 3}
        {     b: 2     }
    ]}";
    pub const APPEND: &str = "{ inner: [
        {a: 1}
        {b: 2}
        {c: 3}
    ]}";
    pub const PREPEND: &str = "{ inner: [
        {c: 3}
        {a: 1}
        {b: 2}
    ]}";
    pub const OVERWRITE: &str = "{ inner: [
        {c: 3}
    ]}";
}

mod list {
    pub const LEFT: &str = "{a: [1, 2, 3]}";
    pub const RIGHT: &str = "{a: [4, 5, 6]}";

    pub const TABLE: &str = "{a: [4, 5, 6]}";
    pub const APPEND: &str = "{a: [1, 2, 3, 4, 5, 6]}";
    pub const PREPEND: &str = "{a: [4, 5, 6, 1, 2, 3]}";
    pub const OVERWRITE: &str = "{a: [4, 5, 6]}";
}

#[rstest]
#[case::s_table_table(table::LEFT, table::RIGHT, None, table::TABLE)]
#[case::s_table_table(table::LEFT, table::RIGHT, Strategy::Table, table::TABLE)]
#[case::s_overwrite_table(table::LEFT, table::RIGHT, Strategy::Overwrite, table::OVERWRITE)]
#[case::s_append_table(table::LEFT, table::RIGHT, Strategy::Append, table::APPEND)]
#[case::s_prepend_table(table::LEFT, table::RIGHT, Strategy::Prepend, table::PREPEND)]
#[case::s_table_list(list::LEFT, list::RIGHT, None, list::TABLE)]
#[case::s_table_list(list::LEFT, list::RIGHT, Strategy::Table, list::TABLE)]
#[case::s_overwrite_list(list::LEFT, list::RIGHT, Strategy::Overwrite, list::OVERWRITE)]
#[case::s_append_list(list::LEFT, list::RIGHT, Strategy::Append, list::APPEND)]
#[case::s_prepend_list(list::LEFT, list::RIGHT, Strategy::Prepend, list::PREPEND)]
#[case::record_nested_with_overwrite(
    "{a: {b: {c: {d: 123, e: 456}}}}",
    "{a: {b: {c: {e: 654, f: 789}}}}",
    None,
    "{a: {b: {c: {d: 123, e: 654, f: 789}}}}"
)]
#[case::single_row_table(
    "[[a]; [{foo: [1, 2, 3]}]]",
    "[[a]; [{bar: [4, 5, 6]}]]",
    None,
    "[[a]; [{foo: [1, 2, 3], bar: [4, 5, 6]}]]"
)]
#[case::multi_row_table(
    "[[a b]; [{inner: {foo: abc}} {inner: {baz: ghi}}]]",
    "[[a b]; [{inner: {bar: def}} {inner: {qux: jkl}}]]",
    None,
    "[[a, b]; [{inner: {foo: abc, bar: def}}, {inner: {baz: ghi, qux: jkl}}]]"
)]
fn merge_deep_tests(
    #[case] left: &str,
    #[case] right: &str,
    #[case] strategy: impl Into<Option<Strategy>>,
    #[case] expected: &str,
) -> Result {
    let mut tester = test();
    let strategy = strategy.into();
    let strategy_is_some = strategy.is_some();

    let () = tester.run_with_data("let left = from nuon", left)?;
    let () = tester.run_with_data("let right = from nuon", right)?;
    let () = tester.run_with_data("let strategy = $in", strategy)?;
    let expected_val: Value = tester.run_with_data("from nuon", expected)?;

    let code = if strategy_is_some {
        "$left | merge deep --strategy=$strategy $right"
    } else {
        "$left | merge deep $right"
    };

    tester.run(code).expect_value_eq(expected_val)
}
