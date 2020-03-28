use nu_test_support::{nu, pipeline};

#[test]
fn headers() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_headers.xlsx
            | get Sheet1
            | headers
            | get Column1
            | from-json"#
    ));
    assert_eq!(actual, "r1c1r2c1")
}
