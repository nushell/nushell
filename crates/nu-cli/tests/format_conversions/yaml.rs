use nu_test_support::{nu, pipeline};

#[test]
fn table_to_yaml_text_and_from_yaml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open appveyor.yml
            | to yaml
            | from yaml
            | get environment.global.PROJECT_NAME
        "#
    ));

    assert_eq!(actual.out, "nushell");
}
