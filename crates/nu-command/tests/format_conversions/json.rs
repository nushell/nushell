use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn table_to_json_text_and_from_json_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sgml_description.json
            | to json
            | from json
            | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
        "#
    ));

    assert_eq!(actual.out, "markup");
}

#[test]
fn table_to_json_float_doesnt_become_int() {
    let actual = nu!(pipeline(
        r#"
            [[a]; [1.0]] | to json | from json | get 0.a | describe
        "#
    ));

    assert_eq!(actual.out, "float")
}

#[test]
fn from_json_text_to_table() {
    Playground::setup("filter_from_json_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {
                    "katz": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "JT", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff", "rusty_luck": 1}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.txt | from json | get katz | get rusty_luck | length "
        );

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn from_json_text_to_table_strict() {
    Playground::setup("filter_from_json_test_1_strict", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {
                    "katz": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "JT", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff", "rusty_luck": 1}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.txt | from json -s | get katz | get rusty_luck | length "
        );

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn from_json_text_recognizing_objects_independently_to_table() {
    Playground::setup("filter_from_json_test_2", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {"name":   "Yehuda", "rusty_luck": 1}
                {"name": "JT", "rusty_luck": 1}
                {"name":   "Andres", "rusty_luck": 1}
                {"name":"GorbyPuff", "rusty_luck": 3}
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open katz.txt
                | from json -o
                | where name == "GorbyPuff"
                | get rusty_luck.0
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn from_json_text_objects_is_stream() {
    Playground::setup("filter_from_json_test_2_is_stream", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {"name":   "Yehuda", "rusty_luck": 1}
                {"name": "JT", "rusty_luck": 1}
                {"name":   "Andres", "rusty_luck": 1}
                {"name":"GorbyPuff", "rusty_luck": 3}
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open katz.txt
                | from json -o
                | describe -n
            "#
        ));

        assert_eq!(actual.out, "stream");
    })
}

#[test]
fn from_json_text_recognizing_objects_independently_to_table_strict() {
    Playground::setup("filter_from_json_test_2_strict", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "katz.txt",
            r#"
                {"name":   "Yehuda", "rusty_luck": 1}
                {"name": "JT", "rusty_luck": 1}
                {"name":   "Andres", "rusty_luck": 1}
                {"name":"GorbyPuff", "rusty_luck": 3}
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open katz.txt
                | from json -o -s
                | where name == "GorbyPuff"
                | get rusty_luck.0
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn table_to_json_text() {
    Playground::setup("filter_to_json_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.txt",
            r#"
                JonAndrehudaTZ,3
                GorbyPuff,100
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.txt
                | lines
                | split column "," name luck
                | select name
                | to json
                | from json
                | get 0
                | get name
            "#
        ));

        assert_eq!(actual.out, "JonAndrehudaTZ");
    })
}

#[test]
fn table_to_json_text_strict() {
    Playground::setup("filter_to_json_test_strict", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.txt",
            r#"
                JonAndrehudaTZ,3
                GorbyPuff,100
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.txt
                | lines
                | split column "," name luck
                | select name
                | to json
                | from json -s
                | get 0
                | get name
            "#
        ));

        assert_eq!(actual.out, "JonAndrehudaTZ");
    })
}

#[test]
fn top_level_values_from_json() {
    for (value, type_name) in [("null", "nothing"), ("true", "bool"), ("false", "bool")] {
        let actual = nu!(r#""{}" | from json | to json"#, value);
        assert_eq!(actual.out, value);
        let actual = nu!(r#""{}" | from json | describe"#, value);
        assert_eq!(actual.out, type_name);
    }
}

#[test]
fn top_level_values_from_json_strict() {
    for (value, type_name) in [("null", "nothing"), ("true", "bool"), ("false", "bool")] {
        let actual = nu!(r#""{}" | from json -s | to json"#, value);
        assert_eq!(actual.out, value);
        let actual = nu!(r#""{}" | from json -s | describe"#, value);
        assert_eq!(actual.out, type_name);
    }
}

#[test]
fn strict_parsing_fails_on_comment() {
    let actual = nu!(r#"'{ "a": 1, /* comment */ "b": 2 }' | from json -s"#);
    assert!(actual.err.contains("error parsing JSON text"));
}

#[test]
fn strict_parsing_fails_on_trailing_comma() {
    let actual = nu!(r#"'{ "a": 1, "b": 2, }' | from json -s"#);
    assert!(actual.err.contains("error parsing JSON text"));
}

#[test]
fn ranges_to_json_as_array() {
    let value = r#"[  1,  2,  3]"#;
    let actual = nu!(r#"1..3 | to json"#);
    assert_eq!(actual.out, value);
}

#[test]
fn unbounded_from_in_range_fails() {
    let actual = nu!(r#"1.. | to json"#);
    assert!(actual.err.contains("Cannot create range"));
}

#[test]
fn inf_in_range_fails() {
    let actual = nu!(r#"inf..5 | to json"#);
    assert!(actual.err.contains("can't convert to countable values"));
    let actual = nu!(r#"5..inf | to json"#);
    assert!(
        actual
            .err
            .contains("Unbounded ranges are not allowed when converting to this format")
    );
    let actual = nu!(r#"-inf..inf | to json"#);
    assert!(actual.err.contains("can't convert to countable values"));
}

#[test]
fn test_indent_flag() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
    r#"
        echo '{ "a": 1, "b": 2, "c": 3 }'
        | from json
        | to json --indent 3
    "#
    ));

    let expected_output = "{   \"a\": 1,   \"b\": 2,   \"c\": 3}";

    assert_eq!(actual.out, expected_output);
}

#[test]
fn test_tabs_indent_flag() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
    r#"
        echo '{ "a": 1, "b": 2, "c": 3 }'
        | from json
        | to json --tabs 2
    "#
    ));

    let expected_output = "{\t\t\"a\": 1,\t\t\"b\": 2,\t\t\"c\": 3}";

    assert_eq!(actual.out, expected_output);
}
