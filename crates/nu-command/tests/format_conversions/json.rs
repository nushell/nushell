use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn table_to_json_text_and_from_json_text_back_into_table() -> Result {
    let code = "
        open sgml_description.json
        | to json
        | from json
        | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("markup")
}

#[test]
fn table_to_json_float_doesnt_become_int() -> Result {
    let code = "[[a]; [1.0]] | to json | from json | get 0.a";
    let outcome: Value = test().run(code)?;
    assert!(matches!(outcome, Value::Float { .. }));
    Ok(())
}

#[test]
fn from_json_text_to_table() -> Result {
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

        let code = "
            open katz.txt
            | from json
            | get katz
            | get rusty_luck
            | length
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq(4)
    })
}

#[test]
fn from_json_text_to_table_strict() -> Result {
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

        let code = "
            open katz.txt
            | from json -s
            | get katz
            | get rusty_luck
            | length
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq(4)
    })
}

#[test]
fn from_json_text_recognizing_objects_independently_to_table() -> Result {
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

        let code = r#"
            open katz.txt
            | from json -o
            | where name == "GorbyPuff"
            | get rusty_luck.0
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn from_json_text_objects_is_stream() -> Result {
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

        let code = "
            open katz.txt
            | from json -o
            | describe -n
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq("stream")
    })
}

#[test]
fn from_json_text_recognizing_objects_independently_to_table_strict() -> Result {
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

        let code = r#"
            open katz.txt
            | from json -o -s
            | where name == "GorbyPuff"
            | get rusty_luck.0
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq(3)
    })
}

#[test]
fn table_to_json_text() -> Result {
    Playground::setup("filter_to_json_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.txt",
            "
                JonAndrehudaTZ,3
                GorbyPuff,100
            ",
        )]);

        let code = r#"
            open sample.txt
            | lines
            | split column "," name luck
            | select name
            | to json
            | from json
            | get 0
            | get name
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("JonAndrehudaTZ")
    })
}

#[test]
fn table_to_json_text_strict() -> Result {
    Playground::setup("filter_to_json_test_strict", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.txt",
            "
                JonAndrehudaTZ,3
                GorbyPuff,100
            ",
        )]);

        let code = r#"
            open sample.txt
            | lines
            | split column "," name luck
            | select name
            | to json
            | from json -s
            | get 0
            | get name
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("JonAndrehudaTZ")
    })
}

#[test]
fn top_level_values_from_json() -> Result {
    for (value, type_name) in [("null", "nothing"), ("true", "bool"), ("false", "bool")] {
        let code = format!(r#""{value}" | from json | to json"#);
        test().run(&code).expect_value_eq(value)?;

        let code = format!(r#""{value}" | from json | describe"#);
        test().run(&code).expect_value_eq(type_name)?;
    }
    Ok(())
}

#[test]
fn top_level_values_from_json_strict() -> Result {
    for (value, type_name) in [("null", "nothing"), ("true", "bool"), ("false", "bool")] {
        let code = format!(r#""{value}" | from json -s | to json"#);
        test().run(&code).expect_value_eq(value)?;

        let code = format!(r#""{value}" | from json -s | describe"#);
        test().run(&code).expect_value_eq(type_name)?;
    }
    Ok(())
}

#[test]
fn strict_parsing_fails_on_comment() -> Result {
    let code = r#"'{ "a": 1, /* comment */ "b": 2 }' | from json -s"#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::Generic(err) => {
            assert_contains("error parsing JSON text", err.msg);
            Ok(())
        }
        other => Err(other.into()),
    }
}

#[test]
fn strict_parsing_fails_on_trailing_comma() -> Result {
    let code = r#"'{ "a": 1, "b": 2, }' | from json -s"#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::Generic(err) => {
            assert_contains("error parsing JSON text", err.msg);
            Ok(())
        }
        other => Err(other.into()),
    }
}

#[test]
fn ranges_to_json_as_array() -> Result {
    let code = "1..3 | to json";
    test().run(code).expect_value_eq("[\n  1,\n  2,\n  3\n]")
}

#[test]
fn unbounded_from_in_range_fails() -> Result {
    let code = "1.. | to json";

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::Generic(err) => {
            assert_contains("Cannot create range", err.error);
            Ok(())
        }
        other => Err(other.into()),
    }
}

#[test]
fn inf_in_range_fails() -> Result {
    let code = "inf..5 | to json";
    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::CannotCreateRange { .. }));

    let code = "5..inf | to json";
    let err = test().run(code).expect_shell_error()?;
    let ShellError::Generic(err) = err else {
        panic!("unexpected err, {err:?}")
    };
    assert_eq!(
        err.msg,
        "Unbounded ranges are not allowed when converting to this format"
    );

    let code = "-inf..inf | to json";
    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::CannotCreateRange { .. }));

    Ok(())
}

#[test]
fn test_indent_flag() -> Result {
    let code = r#"
        echo '{ "a": 1, "b": 2, "c": 3 }'
        | from json
        | to json --indent 3
    "#;

    let expected_output = "{\n   \"a\": 1,\n   \"b\": 2,\n   \"c\": 3\n}";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(expected_output)
}

#[test]
fn test_tabs_indent_flag() -> Result {
    let code = r#"
        echo '{ "a": 1, "b": 2, "c": 3 }'
        | from json
        | to json --tabs 2
    "#;

    let expected_output = "{\n\t\t\"a\": 1,\n\t\t\"b\": 2,\n\t\t\"c\": 3\n}";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(expected_output)
}
