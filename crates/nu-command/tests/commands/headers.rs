use nu_test_support::{nu, pipeline};

#[test]
fn headers_uses_first_row_as_header() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_headers.xlsx
            | get Sheet1
            | headers
            | get header0
            | from json"#
    ));

    assert_eq!(actual.out, "r1c0r2c0")
}

#[test]
fn headers_adds_missing_column_name() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_headers.xlsx
            | get Sheet1
            | headers
            | get Column1
            | from json"#
    ));

    assert_eq!(actual.out, "r1c1r2c1")
}
