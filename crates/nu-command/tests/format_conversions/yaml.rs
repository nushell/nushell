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

#[test]
fn convert_dict_to_yaml_with_boolean_key() {
    let actual = nu!(
        pipeline(
        r#"
            "true: BooleanKey " | from yaml
        "#
    ));
    assert!(actual.out.contains("BooleanKey"));
    assert!(actual.err.is_empty());
}

#[test]
fn convert_dict_to_yaml_with_integer_key() {
    let actual = nu!(
        pipeline(
        r#"
            "200: [] " | from yaml
        "#
    ));

    assert!(actual.out.contains("200"));
    assert!(actual.err.is_empty());
}

#[test]
fn convert_dict_to_yaml_with_integer_floats_key() {
    let actual = nu!(
        pipeline(
        r#"
            "2.11: "1" " | from yaml
        "#
    ));
    assert!(actual.out.contains("2.11"));
    assert!(actual.err.is_empty());
}
