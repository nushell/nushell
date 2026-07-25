//! Unit tests for [`crate::variables`].

use crate::variables::{JSON_MAX_ITEMS, is_table, scalar_preview, short_render, to_json};
use nu_protocol::{Record, Span, Value};
use pretty_assertions::assert_eq;
use rstest::rstest;

fn sp() -> Span {
    Span::unknown()
}
fn s(x: &str) -> Value {
    Value::string(x, sp())
}
fn i(x: i64) -> Value {
    Value::int(x, sp())
}
fn rec(fields: &[(&str, Value)]) -> Value {
    let mut r = Record::new();
    for (k, v) in fields {
        r.push(*k, v.clone());
    }
    Value::record(r, sp())
}

#[rstest]
#[case(i(42), "42")]
#[case(Value::bool(true, Span::unknown()), "true")]
#[case(Value::nothing(Span::unknown()), "null")]
#[case(s("hi"), "\"hi\"")]
fn short_render_scalars(#[case] value: Value, #[case] expected: &str) {
    assert_eq!(short_render(&value), expected);
}

#[test]
fn short_render_list_previews_elements() {
    let v = Value::list(vec![i(1), i(2), i(3)], sp());
    assert_eq!(short_render(&v), "[1, 2, 3]");
    // More than three elements: preview the first three then an ellipsis.
    let long = Value::list((0..50).map(i).collect(), sp());
    assert_eq!(short_render(&long), "[0, 1, 2, …]");
}

#[test]
fn short_render_record_previews_fields() {
    let v = rec(&[("name", s("a.txt")), ("size", i(120))]);
    assert_eq!(short_render(&v), "{name: a.txt, size: 120}");
}

#[test]
fn tables_are_lists_of_records() {
    let row = rec(&[("a", i(1))]);
    let table = Value::list(vec![row.clone(), row], sp());
    assert!(is_table(&table));
    assert_eq!(short_render(&table), "[table 2 rows]");
    // A list of scalars is not a table.
    assert!(!is_table(&Value::list(vec![i(1), i(2)], sp())));
}

#[test]
fn short_render_binary_is_hex() {
    let four = Value::binary(vec![0xde, 0xad, 0xbe, 0xef], sp());
    assert_eq!(short_render(&four), "0x[de ad be ef] (4 bytes)");
    // More than eight bytes: first eight, ellipsis, and the total count.
    let many = Value::binary((0u8..12).collect::<Vec<_>>(), sp());
    assert_eq!(
        short_render(&many),
        "0x[00 01 02 03 04 05 06 07 …] (12 bytes)"
    );
}

#[rstest]
#[case(s("short"), "short")]
#[case(s("abcdefghijklmnop"), "abcdefghijkl…")]
#[case(i(5), "5")]
fn scalar_preview_caps_strings(#[case] value: Value, #[case] expected: &str) {
    assert_eq!(scalar_preview(&value), expected);
}

#[test]
fn to_json_scalars_and_binary_marker() {
    let mut truncated = false;
    assert_eq!(to_json(&i(7), 0, &mut truncated), serde_json::json!(7));
    let bin = Value::binary(vec![0x00, 0xff], sp());
    assert_eq!(
        to_json(&bin, 0, &mut truncated),
        serde_json::json!({ "$nuBinary": "00ff", "length": 2 })
    );
    assert!(!truncated, "small values are not truncated");
}

#[test]
fn to_json_truncates_large_collections() {
    let big = Value::list((0..2000).map(i).collect(), sp());
    let mut truncated = false;
    let json = to_json(&big, 0, &mut truncated);
    assert!(truncated, "the truncated flag is set");
    assert_eq!(json.as_array().unwrap().len(), JSON_MAX_ITEMS);
}
