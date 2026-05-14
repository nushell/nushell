use nu_protocol::{Type, record};
use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;
use rstest::rstest;

const MODULE_SETUP: &str = "
    module spam {
        export const X = 'x'
        export module eggs {
            export const E = 'e'
            export module bacon {
                export const viking = 'eats'
                export module none {}
            }
        }
    }
";

#[test]
fn const_bool() -> Result {
    let code = "
        const x = false
        $x
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
fn const_int() -> Result {
    let code = "
        const x = 10
        $x
    ";

    test().run(code).expect_value_eq(10)
}

#[test]
fn const_float() -> Result {
    let code = "
        const x = 1.234
        $x
    ";

    test().run(code).expect_value_eq(1.234)
}

#[test]
fn const_binary() -> Result {
    let code = "
        const x = 0x[12]
        $x
    ";

    test().run(code).expect_value_eq(Value::test_binary([0x12]))
}

#[test]
fn const_datetime() -> Result {
    let code = "
        const x = 2021-02-27T13:55:40+00:00
        $x
    ";

    let outcome: Value = test().run(code)?;
    let date = outcome.as_date()?.to_rfc3339();
    assert_eq!(date, "2021-02-27T13:55:40+00:00");
    Ok(())
}

#[test]
fn const_list() -> Result {
    let code = "
        const x = [ a b c ]
        $x
    ";

    test().run(code).expect_value_eq(["a", "b", "c"])
}

#[test]
fn const_record() -> Result {
    let code = "
        const x = { a: 10, b: 20, c: 30 }
        $x
    ";

    let expected = record! {
        "a" => Value::test_int(10),
        "b" => Value::test_int(20),
        "c" => Value::test_int(30),
    };

    test().run(code).expect_value_eq(expected)
}

#[test]
fn const_table() -> Result {
    let code = "
        const x = [[a b c]; [10 20 30] [100 200 300]]
        $x | describe
    ";

    test()
        .run(code)
        .expect_value_eq("table<a: int, b: int, c: int>")
}

#[test]
fn const_invalid_table() -> Result {
    let code = "
        const x = [[a b a]; [10 20 30] [100 200 300]]
    ";

    let err = test().run(code).expect_parse_error()?.to_string();
    assert_contains("Record field or table column used twice: a", err);
    Ok(())
}

#[test]
fn const_string() -> Result {
    let code = r#"
        const x = "abc"
        $x
    "#;

    test().run(code).expect_value_eq("abc")
}

#[test]
fn const_string_interpolation_var() -> Result {
    let code = r#"
        const x = 2
        const s = $"($x)"
        $s
    "#;

    test().run(code).expect_value_eq("2")
}

#[test]
fn const_string_interpolation_date() -> Result {
    let code = r#"
        const s = $"(2021-02-27T13:55:40+00:00)"
        $s
    "#;

    let outcome: String = test().locale_en().run(code)?;
    assert_contains("Sat, 27 Feb 2021 13:55:40 +0000", outcome);
    Ok(())
}

#[test]
fn const_string_interpolation_filesize() -> Result {
    let code = r#"
        const s = $"(2kB)"
        $s
    "#;

    test().locale_en().run(code).expect_value_eq("2.0 kB")
}

#[test]
fn const_nothing() -> Result {
    let code = "
      const x = null
      $x | describe
    ";

    test().run(code).expect_value_eq("nothing")
}

#[rstest]
#[case("const x = not false; $x", true)]
#[case("const x = false; const y = not $x; $y", true)]
#[case("const x = not false; const y = not $x; $y", false)]
fn const_unary_operator(#[case] code: &str, #[case] expect: bool) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case("const x = 1 + 2; $x", 3)]
#[case("const x = 1 * 2; $x", 2)]
#[case("const x = 4 / 2; $x", 2.0)]
#[case("const x = 4 mod 3; $x", 1)]
#[case("const x = 5.0 / 2.0; $x", 2.5)]
#[case(r#"const x = "a" + "b"; $x"#, "ab")]
#[case(r#"const x = "a" ++ "b"; $x"#, "ab")]
#[case("const x = [1,2] ++ [3]; $x", [1_i64, 2, 3])]
#[case("const x = 0x[1,2] ++ 0x[3]; $x", Value::test_binary([0x12_u8, 0x03]))]
#[case("const x = 1 < 2; $x", true)]
#[case("const x = (3 * 200) > (2 * 100); $x", true)]
#[case("const x = (3 * 200) < (2 * 100); $x", false)]
#[case("const x = (3 * 200) == (2 * 300); $x", true)]
fn const_binary_operator(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case("const x = 1 / 0; $x", "division by zero")]
#[case("const x = 10 ** 10000000; $x", "pow operation overflowed")]
#[case("const x = 2 ** 62 * 2; $x", "multiply operation overflowed")]
#[case("const x = 1 ++ 0; $x", "operator does not work on values of type")]
fn const_operator_error(#[case] code: &str, #[case] expect: &str) -> Result {
    let err = test().run(code).expect_parse_error()?.to_string();
    assert_contains(expect, err);
    Ok(())
}

#[rstest]
#[case("const x = (1..3); $x | math sum", 6)]
#[case("const x = (1..3); $x | describe", "range")]
fn const_range(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[test]
fn const_subexpression_supported() -> Result {
    let code = "
        const x = ('spam')
        $x
    ";

    test().run(code).expect_value_eq("spam")
}

#[test]
fn const_command_supported() -> Result {
    let code = "
      const x = ('spam' | str length)
      $x
    ";

    test().run(code).expect_value_eq(4)
}

#[test]
fn const_command_unsupported() -> Result {
    let code = "
      const x = (loop { break })
    ";

    let outcome = test().run(code).expect_parse_error()?.to_string();
    assert_contains("not_a_const_command", outcome);
    Ok(())
}

#[test]
fn const_in_scope() -> Result {
    let code = "
      do { const x = 'x'; $x }
    ";

    test().run(code).expect_value_eq("x")
}

#[test]
fn not_a_const_help() -> Result {
    let code = "const x = ('abc' | str length -h)";

    let outcome = test().run(code).expect_parse_error()?.to_string();
    assert_contains("not_a_const_help", outcome);
    Ok(())
}

#[rstest]
#[case("$spam.X", "x")]
#[case("$spam.eggs.E", "e")]
#[case("$spam.eggs.bacon.viking", "eats")]
#[case("'none' in $spam.eggs.bacon", false)]
fn complex_const_export(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    let mut tester = test();

    let () = tester.run(MODULE_SETUP)?;
    let () = tester.run("use spam")?;
    tester.run(code).expect_value_eq(expect)
}

#[rstest]
#[case("$X", "x")]
#[case("$eggs.E", "e")]
#[case("$eggs.bacon.viking", "eats")]
#[case("'none' in $eggs.bacon", false)]
fn complex_const_glob_export(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    let mut tester = test();

    let () = tester.run(MODULE_SETUP)?;
    let () = tester.run("use spam *")?;
    tester.run(code).expect_value_eq(expect)
}

#[test]
fn complex_const_drill_export() -> Result {
    let mut tester = test();

    let () = tester.run(MODULE_SETUP)?;
    let () = tester.run("use spam eggs bacon none")?;
    let err = tester.run("$none").expect_parse_error()?.to_string();
    assert_contains("Variable not found", err);
    Ok(())
}

#[test]
fn complex_const_list_export() -> Result {
    let mut tester = test();

    let () = tester.run(MODULE_SETUP)?;
    let () = tester.run("use spam [X eggs]")?;
    tester
        .run("[$X $eggs.E] | str join ''")
        .expect_value_eq("xe")
}

#[test]
fn exported_const_is_const() -> Result {
    let code = "
        module foo {
            export def main [] { 'foo' }
        }
        module spam {
            export const MOD_NAME = 'foo'
        }

        use spam
        use $spam.MOD_NAME

        foo
    ";

    test().run(code).expect_value_eq("foo")
}

#[rstest]
#[case("$env.SPAM", "xy")]
#[case("spam", "xy")]
fn const_captures_work(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    let module = "
        module spam {
            export const X = 'x'
            const Y = 'y'

            export-env { $env.SPAM = $X + $Y }
            export def main [] { $X + $Y }
        }
    ";

    let mut tester = test();

    let () = tester.run(module)?;
    let () = tester.run("use spam")?;
    tester.run(code).expect_value_eq(expect)
}

#[test]
fn const_captures_in_closures_work() -> Result {
    let code = "
        module foo {
            const a = 'world'
            export def bar [] {
                'hello ' + $a
            }
        }

        use foo

        do { foo bar }
    ";

    test().run(code).expect_value_eq("hello world")
}

#[rstest]
#[case("$X", "x")]
#[case("$eggs.E", "e")]
#[case("$eggs.bacon.viking", "eats")]
#[case("($eggs.bacon not-has 'none')", true)]
fn complex_const_overlay_use(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    let mut tester = test();

    let () = tester.run(MODULE_SETUP)?;
    let () = tester.run("use spam *")?;
    tester.run(code).expect_value_eq(expect)
}

#[ignore = "TODO: `overlay hide` should be possible to use after `overlay use` in the same source unit."]
#[test]
fn overlay_use_hide_in_single_source_unit() -> Result {
    let inp = &[MODULE_SETUP, "overlay use spam", "overlay hide", "$eggs"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("nu::parser::variable_not_found"));
    Ok(())
}

// const implementations of commands without dedicated tests
#[test]
fn describe_const() -> Result {
    let code = "
        const x = 'abc' | describe
        $x
    ";

    test().run(code).expect_value_eq("string")
}

#[test]
fn ignore_const() -> Result {
    let code = r#"
        const x = "spam" | ignore
        $x
    "#;

    test().run(code).expect_value_eq(())
}

#[test]
fn version_const() -> Result {
    let code = "
        const x = version
        $x
    ";

    let _: Value = test().run(code)?;
    Ok(())
}

#[rstest]
#[case("const x = (if 2 < 3 { 'yes!' }); $x", "yes!")]
#[case("const x = (if 5 < 3 { 'yes!' } else { 'no!' }); $x", "no!")]
#[case(
    "const x = (if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' }); $x",
    "no!"
)]
fn if_const(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[test]
fn if_const_error() -> Result {
    let err = test().run("const x = if true ()").expect_parse_error()?;
    assert!(matches!(
        err,
        ParseError::TypeMismatch(Type::Block, Type::Nothing, _)
            | ParseError::TypeMismatchHelp(Type::Block, Type::Nothing, _, _)
    ));

    let err = test()
        .run("const x = if true {foo: bar}")
        .expect_parse_error()?;
    assert!(matches!(
        err,
        ParseError::TypeMismatch(Type::Block, Type::Record(_), _)
            | ParseError::TypeMismatchHelp(Type::Block, Type::Record(_), _, _)
    ));

    let err = test()
        .run("const x = if true {1: 2}")
        .expect_parse_error()?;
    assert!(matches!(
        err,
        ParseError::TypeMismatch(Type::Block, Type::Record(_), _)
            | ParseError::TypeMismatchHelp(Type::Block, Type::Record(_), _, _)
    ));

    Ok(())
}

#[test]
fn const_glob_type() -> Result {
    let code = "
        const x: glob = 'aa'
        $x | describe
    ";

    test().run(code).expect_value_eq("glob")
}

#[rstest]
#[case(
    r####"const x = r#'abcde""fghi"''''jkl'#; $x"####,
    r####"abcde""fghi"''''jkl"####
)]
#[case(
    r####"const x = r##'abcde""fghi"''''#jkl'##; $x"####,
    r####"abcde""fghi"''''#jkl"####
)]
#[case(
    r####"const x = r###'abcde""fghi"'''##'#jkl'###; $x"####,
    r####"abcde""fghi"'''##'#jkl"####
)]
#[case("const x = r#'abc'#; $x", "abc")]
fn const_raw_string(#[case] code: &str, #[case] expect: &str) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[test]
fn const_takes_pipeline() -> Result {
    let code = "
        const list = 'bar_baz_quux' | split row '_'
        $list | length
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
fn const_const() -> Result {
    let code = r#"
        const y = (
            const x = "foo";
            $x + $x
        )
        $y
    "#;

    test().run(code).expect_value_eq("foofoo")?;

    let code = r#"
        const y = (
            const x = "foo";
            $x + $x
        )
        $x
    "#;
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::VariableNotFound(..)));

    Ok(())
}
