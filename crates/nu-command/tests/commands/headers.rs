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
            | to json --raw"#
    ));

    assert_eq!(actual.out, r#"["r1c0","r2c0"]"#)
}

#[test]
fn headers_adds_missing_column_name() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_headers.xlsx
            | get Sheet1
            | headers
            | get column1
            | to json --raw"#
    ));

    assert_eq!(actual.out, r#"["r1c1","r2c1"]"#)
}
