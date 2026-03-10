use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn from_range() -> Result {
    let code = r#"
        echo 1..5 | into string | to json -r
        "#;

    test()
        .run(code)
        .expect_value_eq("[\"1\",\"2\",\"3\",\"4\",\"5\"]")
}

#[test]
fn from_number() -> Result {
    let code = r#"
        echo 5 | into string
        "#;

    test().run(code).expect_value_eq("5")
}

#[test]
fn from_float() -> Result {
    let code = r#"
        echo 1.5 | into string
        "#;

    test().run(code).expect_value_eq("1.5")
}

#[test]
fn from_boolean() -> Result {
    let code = r#"
        echo true | into string
        "#;

    test().run(code).expect_value_eq("true")
}

#[test]
fn from_cell_path() -> Result {
    let code = r#"
        $.test | into string
        "#;

    test().run(code).expect_value_eq("$.test")
}

#[test]
fn from_string() -> Result {
    let code = r#"
        echo "one" | into string
        "#;

    test().run(code).expect_value_eq("one")
}

#[test]
fn from_filename() -> Result {
    Playground::setup("from_filename", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let code = "ls sample.toml | get name | into string | get 0";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("sample.toml")
    })
}

#[test]
fn from_filesize() -> Result {
    Playground::setup("from_filesize", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "sample.toml",
            r#"
                [dependency]
                name = "nu"
            "#,
        )]);

        let code = "ls sample.toml | get size | into string | get 0";
        let expected = if cfg!(windows) { "27 B" } else { "25 B" };

        test().cwd(dirs.test()).run(code).expect_value_eq(expected)
    })
}

#[test]
fn from_float_correct_trailing_zeros() -> Result {
    let code = r#"
        1.23000 | into string -d 3
        "#;

    let outcome: String = test().run(code)?;
    assert_contains("1.230", outcome);
    Ok(())
}

#[test]
fn from_int_float_correct_trailing_zeros() -> Result {
    let code = r#"
        1.00000 | into string -d 3
        "#;

    let outcome: String = test().run(code)?;
    assert_contains("1.000", outcome);
    Ok(())
}

#[test]
fn from_int_float_trim_trailing_zeros() -> Result {
    let code = r#"
        1.00000 | into string | $"($in) flat"
        "#;

    let outcome: String = test().run(code)?;
    assert_contains("1 flat", outcome);
    Ok(())
}

#[test]
fn from_table() -> Result {
    let code = r#"
        echo '[{"name": "foo", "weight": 32.377}, {"name": "bar", "weight": 15.2}]'
        | from json
        | into string weight -d 2
    "#;

    #[derive(Debug, FromValue)]
    struct Outcome {
        weight: String,
    }

    let outcome: Vec<Outcome> = test().run(code)?;
    assert_eq!(outcome[0].weight, "32.38");
    assert_eq!(outcome[1].weight, "15.20");
    Ok(())
}

#[test]
fn from_nothing() -> Result {
    let code = r#"
        null | into string
        "#;

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn int_into_string() -> Result {
    let code = r#"
        10 | into string
        "#;

    test().run(code).expect_value_eq("10")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_0() -> Result {
    let code = r#"
    10 | into string --decimals 0
    "#;

    test().run(code).expect_value_eq("10")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_1() -> Result {
    let code = r#"
    10 | into string --decimals 1
    "#;

    test().run(code).expect_value_eq("10.0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_10() -> Result {
    let code = r#"
    10 | into string --decimals 10
    "#;

    test().run(code).expect_value_eq("10.0000000000")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "de_DE.UTF-8")]
fn int_into_string_decimals_respects_system_locale_de() -> Result {
    let code = r#"
    10 | into string --decimals 1
    "#;

    test().run(code).expect_value_eq("10,0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_respects_system_locale_en() -> Result {
    let code = r#"
    10 | into string --decimals 1
    "#;

    test().run(code).expect_value_eq("10.0")
}
