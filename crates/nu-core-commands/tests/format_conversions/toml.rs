use nu_test_support::{nu, pipeline};

#[test]
fn table_to_toml_text_and_from_toml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | to toml
            | from toml
            | get package.name
        "#
    ));

    assert_eq!(actual.out, "nu");
}
