//! Unit tests for [`crate::variables`].

use crate::variables::{JSON_MAX_ITEMS, scalar_preview, short_render, to_preview_json};
use nu_protocol::{Config, Record, Span, Value};
use pretty_assertions::assert_eq;
use rstest::rstest;

fn sp() -> Span {
    Span::unknown()
}
/// Rendering is config-driven (it defers to `Value::to_abbreviated_string`);
/// defaults keep these assertions independent of the user's `$env.config`.
fn render(value: &Value) -> String {
    short_render(value, &Config::default())
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
    assert_eq!(render(&value), expected);
}

#[test]
fn short_render_list_previews_elements() {
    let v = Value::list(vec![i(1), i(2), i(3)], sp());
    assert_eq!(render(&v), "[1, 2, 3]");
    // More than three elements: preview the first three then an ellipsis.
    let long = Value::list((0..50).map(i).collect(), sp());
    assert_eq!(render(&long), "[0, 1, 2, …]");
}

#[test]
fn short_render_record_previews_fields() {
    let v = rec(&[("name", s("a.txt")), ("size", i(120))]);
    assert_eq!(render(&v), "{name: a.txt, size: 120}");
}

#[test]
fn containers_too_wide_to_preview_fall_back_to_the_shell_shape() {
    // A table: no preview, just the shape — and `to_abbreviated_string`
    // pluralizes, so a single row reads "1 row", not "1 rows".
    let row = rec(&[("a", i(1))]);
    let table = Value::list(vec![row.clone(), row.clone()], sp());
    assert_eq!(render(&table), "[table 2 rows]");
    assert_eq!(render(&Value::list(vec![row], sp())), "[table 1 row]");

    // Lists and records wider than one row collapse the same way. (Elements
    // are capped at 12 chars by `scalar_preview`, so it takes very long
    // numbers to push a list preview past the limit.)
    let wide = Value::list(
        (0..9).map(|n| i(100_000_000_000_000_000 + n)).collect(),
        sp(),
    );
    assert_eq!(render(&wide), "[list 9 items]");
    let wide_rec = rec(&[
        ("first-long-field", s("aaaaaaaaaaaaaaaa")),
        ("second-long-field", s("bbbbbbbbbbbbbbbb")),
        ("third-long-field", s("cccccccccccccccc")),
    ]);
    assert_eq!(render(&wide_rec), "{record 3 fields}");
    assert_eq!(render(&rec(&[("a", i(1))])), "{a: 1}");
}

#[test]
fn short_render_binary_is_hex() {
    let four = Value::binary(vec![0xde, 0xad, 0xbe, 0xef], sp());
    assert_eq!(render(&four), "0x[de ad be ef] (4 bytes)");
    // More than eight bytes: first eight, ellipsis, and the total count.
    let many = Value::binary((0u8..12).collect::<Vec<_>>(), sp());
    assert_eq!(render(&many), "0x[00 01 02 03 04 05 06 07 …] (12 bytes)");
}

#[test]
fn short_render_defers_to_the_shell_for_the_long_tail() {
    // Types with no debugger-specific rendering go through
    // `Value::to_abbreviated_string`, so they read as nushell writes them
    // rather than as Rust `Debug`.
    assert_eq!(
        render(&Value::duration(260_000_000_000, sp())),
        "4min 20sec"
    );
    // Filesize formatting now honours config; the default is metric.
    assert_eq!(render(&Value::filesize(1000, sp())), "1.0 kB");
    assert_eq!(render(&Value::float(1.0, sp())), "1.0");
    // Was `Range(1..10)` — `Value`'s `Debug` — before this went through
    // `to_abbreviated_string`.
    let range = nu_protocol::Range::new(
        i(1),
        i(2),
        i(10),
        nu_protocol::ast::RangeInclusion::Inclusive,
        sp(),
    )
    .expect("valid range");
    assert_eq!(render(&Value::range(range, sp())), "1..10");
}

#[test]
fn short_render_caps_a_long_row() {
    let long = s(&"x".repeat(500));
    let rendered = render(&long);
    assert!(rendered.ends_with("…\" (500 chars)"), "{rendered}");
    assert!(rendered.chars().count() < 140, "{rendered}");
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
    assert_eq!(
        to_preview_json(&i(7), 0, &mut truncated),
        serde_json::json!(7)
    );
    let bin = Value::binary(vec![0x00, 0xff], sp());
    assert_eq!(
        to_preview_json(&bin, 0, &mut truncated),
        serde_json::json!({ "$nuBinary": "00ff", "length": 2 })
    );
    assert!(!truncated, "small values are not truncated");
}

#[test]
fn to_json_truncates_large_collections() {
    let big = Value::list((0..2000).map(i).collect(), sp());
    let mut truncated = false;
    let json = to_preview_json(&big, 0, &mut truncated);
    assert!(truncated, "the truncated flag is set");
    assert_eq!(json.as_array().unwrap().len(), JSON_MAX_ITEMS);
}
