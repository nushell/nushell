#[cfg(feature = "bson")]
#[test]
fn table_to_bson_and_back_into_table() {
    use nu_test_support::{nu, pipeline};
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.bson
            | to bson
            | from bson
            | get root
            | get 1.b
        "#
    ));

    assert_eq!(actual.out, "whel");
}
