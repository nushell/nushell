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

    test()
        .run(code)
        .expect_value_eq("value: 'off'\npath: /dev/stdout\nlisten: 0.0.0.0:8444,0.0.0.0:8445 ssl\n")
}

#[test]
fn convert_strings_with_colons_are_not_corrupted() -> Result {
    let code = "{addr: 'on:80'} | to yaml";

    test().run(code).expect_value_eq("addr: on:80\n")
}

#[test]
fn convert_multiline_string_uses_literal_block_scalar() -> Result {
    let code = "{string: \"Hello\\nworld\"} | to yaml";

    test()
        .run(code)
        .expect_value_eq("string: |-\n  Hello\n  world\n")
}

#[test]
fn convert_multiline_sequence_item_uses_literal_block_scalar() -> Result {
    let code = "[\"Hello\\nworld\"] | to yaml";

    test().run(code).expect_value_eq("- |-\n  Hello\n  world\n")
}

#[test]
fn convert_multiline_string_uses_chomping_indicators() -> Result {
    test()
        .run("{string: \"Hello\\nworld\"} | to yaml")
        .expect_value_eq("string: |-\n  Hello\n  world\n")?;

    test()
        .run("{string: \"Hello\\nworld\\n\"} | to yaml")
        .expect_value_eq("string: |\n  Hello\n  world\n")?;

    test()
        .run("{string: \"Hello\\nworld\\n\\n\"} | to yaml")
        .expect_value_eq("string: |+\n  Hello\n  world\n  \n")
}

#[test]
fn convert_multiline_string_normalizes_crlf() -> Result {
    let code = "{string: \"Hello\\r\\nworld\"} | to yaml";

    test()
        .run(code)
        .expect_value_eq("string: |-\n  Hello\n  world\n")
}

#[test]
fn multiline_string_roundtrips_through_yaml() -> Result {
    let code = "{text: \"foo\\nbar\\n\\n\"} | to yaml | from yaml | get text";

    test().run(code).expect_value_eq("foo\nbar\n\n")
}

#[test]
fn convert_plain_strings_are_not_quoted_when_not_required() -> Result {
    let code = r#"
        '{"name":"kong","kind":"Deployment","env":"KONG_DATABASE","path":"/dev/stdout","addr":"on:80","hash":"abc#def"}'
        | from json
        | to yaml
    "#;

    test().run(code).expect_value_eq(
        "name: kong\nkind: Deployment\nenv: KONG_DATABASE\npath: /dev/stdout\naddr: on:80\nhash: abc#def\n",
    )
}

#[test]
fn convert_strings_are_quoted_when_required_for_plain_scalars() -> Result {
    let code = r#"
        '["off","true","null","0","1.5","0x1","0o7",".inf",".nan","a: b","a #b","- x","? x",": x","[x","{x"," foo","foo "]'
        | from json
        | to yaml
    "#;

    test().run(code).expect_value_eq(
        "- 'off'\n- 'true'\n- 'null'\n- '0'\n- '1.5'\n- '0x1'\n- '0o7'\n- '.inf'\n- '.nan'\n- 'a: b'\n- 'a #b'\n- '- x'\n- '? x'\n- ': x'\n- '[x'\n- '{x'\n- ' foo'\n- 'foo '\n",
    )
}

#[test]
fn convert_keys_are_quoted_only_when_required() -> Result {
    let code = r#"
        '{"kong":"ok","true":"bool-like","0":"numeric-like","a:b":"colon","a b":"space","abc#def":"hash-no-space","a: b":"colon-space","a #b":"hash-with-space"}'
        | from json
        | to yaml
    "#;

    test().run(code).expect_value_eq(
        "kong: ok\n'true': bool-like\n'0': numeric-like\na:b: colon\na b: space\nabc#def: hash-no-space\n'a: b': colon-space\n'a #b': hash-with-space\n",
    )
}
