use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};
use pretty_assertions::assert_matches;

#[test]
fn from_range() -> Result {
    let code = "
        echo 1..5 | into string | to json -r
        ";

    test()
        .run(code)
        .expect_value_eq("[\"1\",\"2\",\"3\",\"4\",\"5\"]")
}

#[test]
fn from_number() -> Result {
    let code = "
        echo 5 | into string
        ";

    test().run(code).expect_value_eq("5")
}

#[test]
fn from_float() -> Result {
    let code = "
        echo 1.5 | into string
        ";

    test().run(code).expect_value_eq("1.5")
}

#[test]
fn from_nan() -> Result {
    let code = "
        echo nan | into string
    ";

    test().run(code).expect_value_eq("NaN")
}

#[test]
fn from_inf() -> Result {
    let code = "
        echo inf | into string
    ";

    test().run(code).expect_value_eq("inf")
}

#[test]
fn from_negative_zero() -> Result {
    let code = "
        echo -- -0.0 | into string
    ";

    test().run(code).expect_value_eq("-0")
}

#[test]
fn from_int_negative_zero() -> Result {
    let code = "
        echo -- -0 | into string
    ";

    test().run(code).expect_value_eq("0")
}

#[test]
fn from_boolean() -> Result {
    let code = "
        echo true | into string
        ";

    test().run(code).expect_value_eq("true")
}

#[test]
fn from_cell_path() -> Result {
    let code = "
        $.test | into string
        ";

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
    let code = "
        1.23000 | into string -d 3
        ";

    let outcome: String = test().run(code)?;
    assert_contains("1.230", outcome);
    Ok(())
}

#[test]
fn from_int_float_correct_trailing_zeros() -> Result {
    let code = "
        1.00000 | into string -d 3
        ";

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
fn from_float_rounding() -> Result {
    let code = "
        1.23450 | into string -d 3
    ";

    test().run(code).expect_value_eq("1.235")
}

#[test]
fn from_float_int_rounding() -> Result {
    let code = "
        1.995 | into string -d 2
    ";

    test().run(code).expect_value_eq("2.00")
}

#[test]
fn from_float_neg_rounding() -> Result {
    let code = "
        -1.23450 | into string -d 3
    ";

    test().run(code).expect_value_eq("-1.235")
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
    let code = "
        null | into string
        ";

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn from_semver() -> Result {
    test()
        .run("'1.2.3' | into semver | into string")
        .expect_value_eq("1.2.3")
}

#[test]
fn from_semver_let_binding() -> Result {
    test()
        .run(r#"let s = "1.0.0" | into semver | into string; $s"#)
        .expect_value_eq("1.0.0")
}

#[test]
fn from_semver_let_binding_is_string() -> Result {
    test()
        .run(r#"let s = "1.0.0" | into semver | into string; $s | describe"#)
        .expect_value_eq("string")
}

#[test]
fn from_semver_path_join() -> Result {
    test()
        .run(
            r#""versions" | path join ("1.115.0" | into semver | into string) | path split | last"#,
        )
        .expect_value_eq("1.115.0")
}

#[test]
fn from_semver_range_let_binding() -> Result {
    test()
        .run(r#"let s = ">=1.0.0" | into semver-range | into string; $s"#)
        .expect_value_eq(">=1.0.0")
}

#[test]
fn int_into_string() -> Result {
    let code = "
        10 | into string
        ";

    test().run(code).expect_value_eq("10")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_0() -> Result {
    let code = "
    10 | into string --decimals 0
    ";

    test().run(code).expect_value_eq("10")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_1() -> Result {
    let code = "
    10 | into string --decimals 1
    ";

    test().run(code).expect_value_eq("10.0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_10() -> Result {
    let code = "
    10 | into string --decimals 10
    ";

    test().run(code).expect_value_eq("10.0000000000")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "de_DE.UTF-8")]
fn int_into_string_decimals_respects_system_locale_de() -> Result {
    let code = "
    10 | into string --decimals 1
    ";

    test().run(code).expect_value_eq("10,0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_decimals_respects_system_locale_en() -> Result {
    let code = "
    10 | into string --decimals 1
    ";

    test().run(code).expect_value_eq("10.0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "de_DE.UTF-8")]
fn float_into_string_decimals_respects_system_locale_de() -> Result {
    let code = "
    10.0 | into string --decimals 1
    ";

    test().run(code).expect_value_eq("10,0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_grouping_respects_system_locale_en() -> Result {
    let code = "
    1234 | into string --group-digits
    ";

    test().run(code).expect_value_eq("1,234")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn float_into_string_decimals_respects_system_locale_en() -> Result {
    let code = "
    10.0 | into string --decimals 1
    ";

    test().run(code).expect_value_eq("10.0")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "de_DE.UTF-8")]
fn int_into_string_grouping_respects_system_locale_de() -> Result {
    let code = "
    1234 | into string --group-digits
    ";

    test().run(code).expect_value_eq("1.234")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "de_DE.UTF-8")]
fn float_into_string_grouping_respects_system_locale_de() -> Result {
    let code = "
    1234.0 | into string --group-digits
    ";

    test().run(code).expect_value_eq("1.234")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "hi_IN.UTF-8")]
fn int_into_string_grouping_respects_system_locale_in() -> Result {
    let code = "
    123456 | into string --group-digits
    ";

    test().run(code).expect_value_eq("1,23,456")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "hi_IN.UTF-8")]
fn float_into_string_grouping_respects_system_locale_in() -> Result {
    let code = "
    123456.0 | into string --group-digits
    ";

    test().run(code).expect_value_eq("1,23,456")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_negative_grouping() -> Result {
    let code = "
    -123456 | into string --group-digits
    ";

    test().run(code).expect_value_eq("-123,456")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn float_into_string_negative_grouping() -> Result {
    let code = "
    -123456.0 | into string --group-digits
    ";

    test().run(code).expect_value_eq("-123,456")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn float_into_string_negative_grouping_three_chars() -> Result {
    let code = "
    -123.45 | into string --group-digits
    ";

    test().run(code).expect_value_eq("-123.45")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "hi_IN.UTF-8")]
fn float_into_string_negative_grouping_three_chars_in() -> Result {
    let code = "
    -123.45 | into string --group-digits
    ";

    test().run(code).expect_value_eq("-123.45")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "ar_AE.UTF-8")]
fn int_into_string_negative_respects_system_locale_ar() -> Result {
    let code = "
    -123456 | into string
    ";

    test().run(code).expect_value_eq("\u{61c}-123456")
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "ar_AE.UTF-8")]
fn float_into_string_negative_respects_system_locale_ar() -> Result {
    let code = "
    -123456.0 | into string
    ";

    test().run(code).expect_value_eq("\u{61c}-123456")
}

#[test]
fn float_into_string_decimals_above_u16_range_errors() -> Result {
    let code = "
    1.5 | into string -d 2147483648
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::incorrect_value")
}

#[test]
fn float_into_string_decimals_wrapping_to_small_exponent_errors() -> Result {
    let code = "
    1.5 | into string -d 4294967296
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::incorrect_value")
}

#[test]
fn int_into_string_decimals_above_u16_range_errors() -> Result {
    let code = "
    5 | into string -d 70000
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::incorrect_value")
}

#[test]
fn filesize_into_string_group_digits_decimals_above_u16_range_errors() -> Result {
    let code = "
    1kB | into string --group-digits -d 70000
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::incorrect_value")
}

#[test]
fn float_into_string_large_decimals() -> Result {
    let code = "
    1.5 | into string -d 400
    ";

    let result = format!("1.5{}", "0".repeat(399));
    test().run(code).expect_value_eq(result)
}

#[test]
fn float_into_string_large_negative_decimals() -> Result {
    let code = "
    -1.5 | into string -d 400
    ";

    let result = format!("-1.5{}", "0".repeat(399));
    test().run(code).expect_value_eq(result)
}

#[test]
fn float_into_string_precision_overflow() -> Result {
    let code = "
    1e300 | into string -d 10
    ";

    let result = "1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160.0000000000";
    test().run(code).expect_value_eq(result)
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn int_into_string_i64_min() -> Result {
    let code = "
    -9223372036854775808 | into string
    ";

    test().run(code).expect_value_eq("-9223372036854775808")
}

#[test]
fn from_binary_errors_with_decode_hint() -> Result {
    let code = "
    0x[31 32 33] | into string
    ";

    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, help: Some(help), .. }
            if to_type == "string" && from_type == "binary" && help.contains("`decode`")
    );
    Ok(())
}

#[test]
fn help_says_binary_needs_decode() -> Result {
    let code = "
    help into string | str contains 'decode'
    ";

    test().run(code).expect_value_eq(true)
}
