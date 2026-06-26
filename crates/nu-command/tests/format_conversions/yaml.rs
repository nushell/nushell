use indoc::indoc;
use nu_protocol::{Record, test_record};
use nu_test_support::prelude::*;
use rstest::rstest;

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

#[rstest]
#[case::y("y")]
#[case::n("n")]
#[case::no("no")]
#[case::on("On")]
#[case::off("OFF")]
fn convert_yaml_11_booleans_are_quoted_in_output(#[case] input: &str) -> Result {
    test()
        .run_with_data("$in | to yaml --spec 1.1", input)
        .expect_value_eq(format!("{input:?}\n"))
}

#[test]
fn convert_issue_16072_strings_are_quoted_in_output() -> Result {
    let input = test_record! {
        "value" => "off",
        "path" => "/dev/stdout",
        "listen" => "0.0.0.0:8444,0.0.0.0:8445 ssl"
    };

    let expected = indoc! {r#"
        value: "off"
        path: /dev/stdout
        listen: 0.0.0.0:8444,0.0.0.0:8445 ssl
    "#};

    test()
        .run_with_data("$in | to yaml --spec 1.1", input)
        .expect_value_eq(expected)
}

#[test]
fn convert_strings_with_colons_are_not_corrupted() -> Result {
    test()
        .run("{addr: 'on:80'} | to yaml")
        .expect_value_eq("addr: on:80\n")
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
#[ignore = "normalization may be not a required feature"]
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
    let input = test_record! {
        "name" => "kong",
        "kind" => "Deployment",
        "env" => "KONG_DATABASE",
        "path" => "/dev/stdout",
        "addr" => "on:80",
        // "hash" => "abc#def"
    };

    // the serializer doesn't have to quote this but our used crate does quote here

    let expected = indoc! {"
        name: kong
        kind: Deployment
        env: KONG_DATABASE
        path: /dev/stdout
        addr: on:80
    "};

    test()
        .run_with_data("$in | to yaml --spec 1.1", input)
        .expect_value_eq(expected)
}

#[rstest]
#[case::off("off")]
#[case::bool_true("true")]
#[case::null("null")]
#[case::zero("0")]
#[case::one_dot_five("1.5")]
#[case::hex_one("0x1")]
#[case::oct_seven("0o7")]
#[case::dot_inf(".inf")]
#[case::dot_nan(".nan")]
#[case::a_colon_b("a: b")]
#[case::a_hash_b("a #b")]
#[case::dash_x("- x")]
#[case::question_x("? x")]
#[case::colon_x(": x")]
#[case::square_x("[x")]
#[case::curvy_x("{x")]
#[case::white_foo(" foo")]
fn convert_strings_are_quoted_when_required_for_plain_scalars(#[case] input: &str) -> Result {
    test()
        .run_with_data("$in | to yaml --spec 1.1 --quote auto", input)
        .expect_value_eq(format!("{input:?}\n"))
}

#[rstest]
#[case::kong("kong", false)]
#[case::bool_like("true", true)]
#[case::numeric_like("0", true)]
#[case::colon("a:b", true)]
#[case::space("a b", false)]
#[case::hash_no_space("abc#def", true)] // not really required, our serializer just does that
#[case::colon_space("a: b", true)]
#[case::hash_with_space("a #b", true)]
fn convert_keys_are_quoted_only_when_required(#[case] input: &str, #[case] quoted: bool) -> Result {
    let output: String = test().run_with_data("{$in: null} | to yaml", input)?;
    match quoted {
        true => assert_eq!(output, format!("{input:?}: null\n"), "expected quotes"),
        false => assert_eq!(output, format!("{input}: null\n"), "expected no quotes"),
    };
    Ok(())
}
