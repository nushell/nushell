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
