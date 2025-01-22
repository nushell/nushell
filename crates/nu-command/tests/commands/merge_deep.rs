use nu_test_support::nu;

#[test]
fn table_strategy_table() {
    assert_eq!(
        nu!(
            "{} | merge deep {} | to nuon",
            "{inner: [{a: 1}, {b: 2}]}",
            "{inner: [{c: 3}]}"
        )
        .out,
        "{inner: [{a: 1, c: 3}, {b: 2}]}"
    )
}

#[test]
fn table_strategy_list() {
    assert_eq!(
        nu!(
            "{} | merge deep {} | to nuon",
            "{a: [1, 2, 3]}",
            "{a: [4, 5, 6]}"
        )
        .out,
        "{a: [4, 5, 6]}"
    )
}

#[test]
fn overwrite_strategy_table() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=overwrite {} | to nuon",
            "{inner: [{a: 1}, {b: 2}]}",
            "{inner: [[c]; [3]]}"
        )
        .out,
        "{inner: [[c]; [3]]}"
    )
}

#[test]
fn overwrite_strategy_list() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=overwrite {} | to nuon",
            "{a: [1, 2, 3]}",
            "{a: [4, 5, 6]}"
        )
        .out,
        "{a: [4, 5, 6]}"
    )
}

#[test]
fn append_strategy_table() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=append {} | to nuon",
            "{inner: [{a: 1}, {b: 2}]}",
            "{inner: [{c: 3}]}"
        )
        .out,
        "{inner: [{a: 1}, {b: 2}, {c: 3}]}"
    )
}

#[test]
fn append_strategy_list() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=append {} | to nuon",
            "{inner: [1, 2, 3]}",
            "{inner: [4, 5, 6]}"
        )
        .out,
        "{inner: [1, 2, 3, 4, 5, 6]}"
    )
}

#[test]
fn prepend_strategy_table() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=prepend {} | to nuon",
            "{inner: [{a: 1}, {b: 2}]}",
            "{inner: [{c: 3}]}"
        )
        .out,
        "{inner: [{c: 3}, {a: 1}, {b: 2}]}"
    )
}

#[test]
fn prepend_strategy_list() {
    assert_eq!(
        nu!(
            "{} | merge deep --strategy=prepend {} | to nuon",
            "{inner: [1, 2, 3]}",
            "{inner: [4, 5, 6]}"
        )
        .out,
        "{inner: [4, 5, 6, 1, 2, 3]}"
    )
}

#[test]
fn record_nested_with_overwrite() {
    assert_eq!(
        nu!(
            "{} | merge deep {} | to nuon",
            "{a: {b: {c: {d: 123, e: 456}}}}",
            "{a: {b: {c: {e: 654, f: 789}}}}"
        )
        .out,
        "{a: {b: {c: {d: 123, e: 654, f: 789}}}}"
    )
}

#[test]
fn single_row_table() {
    assert_eq!(
        nu!(
            "{} | merge deep {} | to nuon",
            "[[a]; [{foo: [1, 2, 3]}]]",
            "[[a]; [{bar: [4, 5, 6]}]]"
        )
        .out,
        "[[a]; [{foo: [1, 2, 3], bar: [4, 5, 6]}]]"
    )
}

#[test]
fn multi_row_table() {
    assert_eq!(
        nu!(
            "{} | merge deep {} | to nuon ",
            "[[a b]; [{inner: {foo: abc}} {inner: {baz: ghi}}]]",
            "[[a b]; [{inner: {bar: def}} {inner: {qux: jkl}}]]"
        )
        .out,
        "[[a, b]; [{inner: {foo: abc, bar: def}}, {inner: {baz: ghi, qux: jkl}}]]"
    )
}
