use std::fmt::Display;

use nu_test_support::nu;
use rstest::rstest;

struct Strategy<'a>(Option<&'a str>);

impl<'a> Display for Strategy<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .map(|s| write!(f, "--strategy={s}"))
            .unwrap_or(Ok(()))
    }
}

const TABLE_LEFT: &str = "{inner: [{a: 1}, {b: 2}]}";
const TABLE_RIGHT: &str = "{inner: [{c: 3}]}";

const LIST_LEFT: &str = "{a: [1, 2, 3]}";
const LIST_RIGHT: &str = "{a: [4, 5, 6]}";

#[rstest]
#[case::s_table_table(None, TABLE_LEFT, TABLE_RIGHT, "{inner: [{a: 1, c: 3}, {b: 2}]}")]
#[case::s_table_list(None, LIST_LEFT, LIST_RIGHT, "{a: [4, 5, 6]}")]
#[case::s_overwrite_table("overwrite", TABLE_LEFT, TABLE_RIGHT, "{inner: [[c]; [3]]}")]
#[case::s_overwrite_list("overwrite", LIST_LEFT, LIST_RIGHT, "{a: [4, 5, 6]}")]
#[case::s_append_table("append", TABLE_LEFT, TABLE_RIGHT, "{inner: [{a: 1}, {b: 2}, {c: 3}]}")]
#[case::s_append_list("append", LIST_LEFT, LIST_RIGHT, "{a: [1, 2, 3, 4, 5, 6]}")]
#[case::s_prepend_table(
    "prepend",
    TABLE_LEFT,
    TABLE_RIGHT,
    "{inner: [{c: 3}, {a: 1}, {b: 2}]}"
)]
#[case::s_prepend_list("prepend", LIST_LEFT, LIST_RIGHT, "{a: [4, 5, 6, 1, 2, 3]}")]
#[case::record_nested_with_overwrite(
    None,
    "{a: {b: {c: {d: 123, e: 456}}}}",
    "{a: {b: {c: {e: 654, f: 789}}}}",
    "{a: {b: {c: {d: 123, e: 654, f: 789}}}}"
)]
#[case::single_row_table(
    None,
    "[[a]; [{foo: [1, 2, 3]}]]",
    "[[a]; [{bar: [4, 5, 6]}]]",
    "[[a]; [{foo: [1, 2, 3], bar: [4, 5, 6]}]]"
)]
#[case::multi_row_table(
    None,
    "[[a b]; [{inner: {foo: abc}} {inner: {baz: ghi}}]]",
    "[[a b]; [{inner: {bar: def}} {inner: {qux: jkl}}]]",
    "[[a, b]; [{inner: {foo: abc, bar: def}}, {inner: {baz: ghi, qux: jkl}}]]"
)]
fn merge_deep_tests<'a>(
    #[case] strategy: impl Into<Option<&'a str>>,
    #[case] left: &str,
    #[case] right: &str,
    #[case] expected: &str,
) {
    let strategy = Strategy(strategy.into());
    let actual = nu!(format!("{left} | merge deep {strategy} {right} | to nuon"));
    assert_eq!(actual.out, expected)
}
