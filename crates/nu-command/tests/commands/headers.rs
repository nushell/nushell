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

#[test]
fn headers_invalid_column_type_empty_record() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [{}, 2], [3,4] ]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}

#[test]
fn headers_invalid_column_type_record() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [1 ($nu.scope)] [2 2]]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}

#[test]
fn headers_invalid_column_type_array() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [[f,g], 2], [3,4] ]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}

#[test]
fn headers_invalid_column_type_range() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [(1..5), 2], [3,4] ]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}

#[test]
fn headers_invalid_column_type_duration() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [((date now) - (date now)), 2], [3,4] ]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}

#[test]
fn headers_invalid_column_type_binary() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            [[a b]; [("aa" | into binary), 2], [3,4] ]
            | headers"#
    ));

    assert!(actual
        .err
        .contains("needs compatible type: Null, String, Bool, Float, Int"));
}
