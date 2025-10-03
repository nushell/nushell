use nu_test_support::nu;

#[test]
fn table_to_yaml_text_and_from_yaml_text_back_into_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open appveyor.yml
        | to yaml
        | from yaml
        | get environment.global.PROJECT_NAME
    "#);

    assert_eq!(actual.out, "nushell");
}

#[test]
fn table_to_yml_text_and_from_yml_text_back_into_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open appveyor.yml
        | to yml
        | from yml
        | get environment.global.PROJECT_NAME
    "#);

    assert_eq!(actual.out, "nushell");
}

#[test]
fn convert_dict_to_yaml_with_boolean_key() {
    let actual = nu!(r#"
        "true: BooleanKey " | from yaml
    "#);
    assert!(actual.out.contains("BooleanKey"));
    assert!(actual.err.is_empty());
}

#[test]
fn convert_dict_to_yaml_with_integer_key() {
    let actual = nu!(r#"
        "200: [] " | from yaml
    "#);

    assert!(actual.out.contains("200"));
    assert!(actual.err.is_empty());
}

#[test]
fn convert_dict_to_yaml_with_integer_floats_key() {
    let actual = nu!(r#"
        "2.11: "1" " | from yaml
    "#);
    assert!(actual.out.contains("2.11"));
    assert!(actual.err.is_empty());
}

#[test]
#[ignore]
fn convert_bool_to_yaml_in_yaml_spec_1_2() {
    let actual = nu!(r#"
        [y n no On OFF True true false] | to yaml
    "#);

    assert_eq!(
        actual.out,
        "- 'y'- 'n'- 'no'- 'On'- 'OFF'- 'True'- true- false"
    );
    assert!(actual.err.is_empty());
}
