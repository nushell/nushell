use nu_protocol::Record;
use nu_test_support::prelude::*;

#[test]
fn table_to_yaml_text_and_from_yaml_text_back_into_table() -> Result {
    let code = "
        open appveyor.yml
        | to yaml
        | from yaml
        | get environment.global.PROJECT_NAME
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("nushell")
}

#[test]
fn table_to_yml_text_and_from_yml_text_back_into_table() -> Result {
    let code = "
        open appveyor.yml
        | to yml
        | from yml
        | get environment.global.PROJECT_NAME
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("nushell")
}

#[test]
fn convert_dict_to_yaml_with_boolean_key() -> Result {
    let code = r#""true: BooleanKey " | from yaml"#;

    let outcome: Record = test().run(code)?;
    assert!(outcome.columns().any(|col| col == "true"));
    Ok(())
}

#[test]
fn convert_dict_to_yaml_with_integer_key() -> Result {
    let code = r#""200: [] " | from yaml"#;

    let outcome: Record = test().run(code)?;
    assert!(outcome.columns().any(|col| col == "200"));
    Ok(())
}

#[test]
fn convert_dict_to_yaml_with_integer_floats_key() -> Result {
    let code = r#""2.11: "1" " | from yaml"#;

    let outcome: Record = test().run(code)?;
    assert!(outcome.columns().any(|col| col == "2.11"));
    Ok(())
}

#[test]
fn convert_yaml_11_booleans_are_quoted_in_output() -> Result {
    let code = "[y n no On OFF] | to yaml";

    test()
        .run(code)
        .expect_value_eq("- 'y'\n- 'n'\n- 'no'\n- 'On'\n- 'OFF'\n")
}

#[test]
fn convert_issue_16072_strings_are_quoted_in_output() -> Result {
    let code = r#"
        '{
            "value": "off",
            "path": "/dev/stdout",
            "listen": "0.0.0.0:8444,0.0.0.0:8445 ssl"
        }'
        | from json
        | to yaml
    "#;

    test().run(code).expect_value_eq(
        "value: 'off'\npath: '/dev/stdout'\nlisten: '0.0.0.0:8444,0.0.0.0:8445 ssl'\n",
    )
}

#[test]
fn convert_strings_with_colons_are_not_corrupted() -> Result {
    let code = "{addr: 'on:80'} | to yaml";

    test().run(code).expect_value_eq("addr: 'on:80'\n")
}
