use nu_protocol::test_table;
use nu_test_support::prelude::*;

#[test]
fn counter_clockwise() -> Result {
    let table = test_table![
        ["col1", "col2", "EXPECTED"];
        [ "---",  "|||",      "XX1"],
        [ "---",  "|||",      "XX2"],
        [ "---",  "|||",      "XX3"],
    ];
    let expected = test_table![
        [  "column0", "column1", "column2", "column3"];
        [ "EXPECTED",     "XX1",     "XX2",     "XX3"],
        [     "col2",     "|||",     "|||",     "|||"],
        [     "col1",     "---",     "---",     "---"],
    ];

    test()
        .run_with_data("rotate --ccw", table)
        .expect_value_eq(expected)
}

#[test]
fn clockwise() -> Result {
    let table = test_table![
        ["col1", "col2", "EXPECTED"];
        [ "---",  "|||",      "XX1"],
        [ "---",  "|||",      "XX2"],
        [ "---",  "|||",      "XX3"],
    ];
    let expected = test_table![
        [ "column0", "column1", "column2",  "column3"];
        [     "---",     "---",     "---",     "col1"],
        [     "|||",     "|||",     "|||",     "col2"],
        [     "XX3",     "XX2",     "XX1", "EXPECTED"],
    ];

    test()
        .run_with_data("rotate", table)
        .expect_value_eq(expected)
}

#[test]
fn different_cols_vals_err() -> Result {
    test()
        .run("[[[one], [two, three]]] | first | rotate")
        .expect_error_code_eq("nu::shell::record_cols_vals_mismatch")
}
