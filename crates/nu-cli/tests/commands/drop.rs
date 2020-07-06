use nu_test_support::{nu, pipeline};

#[test]
fn drop_rows() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"echo '[{"foo": 3}, {"foo": 8}, {"foo": 4}]' | from json | drop 2 | get foo | math sum | echo $it"#
    );

    assert_eq!(actual.out, "3");
}

#[test]
fn drop_more_rows_than_table_has() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        date | drop 50 | count
        "#
    ));

    assert_eq!(actual.out, "0");
}
