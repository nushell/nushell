use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn headers_uses_first_row_as_header() -> Result {
    let code = "
        open sample_headers.xlsx --raw
        | from xlsx --noheaders
        | get Sheet1
        | headers
        | get header0
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(["r1c0", "r2c0"])
}

#[test]
fn headers_adds_missing_column_name() -> Result {
    let code = "
        open sample_headers.xlsx --raw
        | from xlsx --noheaders
        | get Sheet1
        | headers
        | get column1
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(["r1c1", "r2c1"])
}

#[test]
fn headers_handles_missing_values() -> Result {
    let code = "
        [{x: a, y: b}, {x: 1, y: 2}, {x: 1, z: 3}]
        | headers
    ";

    test().run(code).expect_value_eq(test_value!([
        { a: 1, b: 2 },
        { a: 1 },
    ]))
}

#[rstest]
#[case::empty_record("[[a b]; [{}, 2], [3, 4]] | headers")]
#[case::record("[[a b]; [1 (scope aliases)] [2 2]] | headers")]
#[case::array("[[a b]; [[f, g], 2], [3, 4]] | headers")]
#[case::range("[[a b]; [(1..5), 2], [3, 4]] | headers")]
#[case::duration("[[a b]; [((date now) - (date now)), 2], [3, 4]] | headers")]
#[case::binary(r#"[[a b]; [("aa" | into binary), 2], [3, 4]] | headers"#)]
fn headers_invalid_column_type(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::TypeMismatch { err_message, .. }
            if err_message == "needs compatible type: Null, String, Bool, Float, Int"
    );
    Ok(())
}

#[test]
fn headers_renames_repeated_names() -> Result {
    test()
        .run("[[a b c]; [x x x] [1 2 3]] | headers | to nuon")
        .expect_value_eq(r#"[[x, "x_1", "x_2"]; [1, 2, 3]]"#)
}

#[test]
fn headers_repeated_name_skips_existing_suffix() -> Result {
    // A real `x_1` column keeps its name; the repeated `x` moves past it.
    test()
        .run("[[a b c]; [x x x_1] [1 2 3]] | headers | to nuon")
        .expect_value_eq(r#"[[x, "x_2", "x_1"]; [1, 2, 3]]"#)
}

#[test]
fn headers_blank_names_collide_with_literal_column_name() -> Result {
    // Blank cells are named by position first (`column0`, `column1`), then a
    // literal `column0` in the header row is renamed rather than duplicated.
    test()
        .run(r#"[[a b c]; ["" "" column0] [1 2 3]] | headers | to nuon"#)
        .expect_value_eq(r#"[["column0", "column1", "column0_1"]; [1, 2, 3]]"#)
}

#[test]
fn headers_renamed_columns_apply_to_every_row() -> Result {
    test()
        .run("[[a b]; [x x] [1 2] [3 4]] | headers | to nuon")
        .expect_value_eq(r#"[[x, "x_1"]; [1, 2], [3, 4]]"#)
}

#[test]
fn headers_non_string_names_are_deduped() -> Result {
    // Int and float header cells become "1" and "1.0"; the repeated "1" gets a suffix.
    test()
        .run("[[a b c]; [1 1.0 1] [1 2 3]] | headers | columns | to nuon")
        .expect_value_eq(r#"["1", "1.0", "1_1"]"#)
}

#[test]
fn headers_null_name_collides_with_literal() -> Result {
    // A null header cell is named `column0`, so a literal `column0` in the next column is renamed.
    test()
        .run("[[a b]; [null column0] [1 2]] | headers | columns | to nuon")
        .expect_value_eq(r#"["column0", "column0_1"]"#)
}

#[test]
fn headers_reordered_and_missing_columns_map_by_name() -> Result {
    test()
        .run("[{a: x, b: x}, {b: 2, a: 1}, {a: 5}] | headers | to nuon")
        .expect_value_eq(r#"[{"x_1": 2, x: 1}, {x: 5}]"#)
}

#[test]
fn headers_renamed_column_is_reachable_by_name() -> Result {
    test()
        .run("[[a b]; [x x] [1 2]] | headers | get x_1 | to nuon")
        .expect_value_eq("[2]")
}
