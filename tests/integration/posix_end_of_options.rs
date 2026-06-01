/// Tests for POSIX `--` end-of-options delimiter support
///
/// This test file validates that `--` stops all flag parsing and treats
/// all following arguments as operands, even if they start with `-` or `--`.
/// This aligns with POSIX Guideline 10:
/// https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html#tag_12_02
use nu_test_support::prelude::*;

#[test]
fn echo_with_end_of_options_stops_flag_parsing() -> Result {
    test()
        .run("echo -- -n hello")
        .expect_value_eq(["-n", "hello"])
}

#[test]
fn echo_with_end_of_options_multiple_dashes() -> Result {
    test()
        .run("echo -- --foo -bar baz")
        .expect_value_eq(["--foo", "-bar", "baz"])
}

#[test]
fn echo_with_end_of_options_alone() -> Result {
    // echo with no positional args after -- returns an empty string
    test().run("echo --").expect_value_eq("")
}

#[test]
fn echo_before_and_after_end_of_options() -> Result {
    test()
        .run("echo arg1 -- -arg2")
        .expect_value_eq(["arg1", "-arg2"])
}

#[test]
fn custom_command_with_end_of_options_and_positional() -> Result {
    test()
        .run(
            "
            def my_cmd [x y] { [$x $y] }
            my_cmd -- -value1 -value2
        ",
        )
        .expect_value_eq(vec!["-value1", "-value2"])
}

#[test]
fn custom_command_with_end_of_options_and_rest() -> Result {
    test()
        .run(
            "
            def my_cmd [x ...rest] { [$x] ++ $rest }
            my_cmd first -- -second -third
        ",
        )
        .expect_value_eq(vec!["first", "-second", "-third"])
}

#[test]
fn custom_command_with_flags_before_end_of_options() -> Result {
    // --flag is a boolean switch; value1 becomes the first positional x
    test()
        .run(
            "
            def my_cmd [--flag x y z] { [$x $y $z] }
            my_cmd --flag value1 -- -value2 -value3
        ",
        )
        .expect_value_eq(vec!["value1", "-value2", "-value3"])
}

#[test]
fn custom_command_with_optional_positional_and_end_of_options() -> Result {
    test()
        .run(
            "
            def my_cmd [x y?] { if ($y == null) { $x } else { [$x $y] } }
            my_cmd -- -value1 -value2
        ",
        )
        .expect_value_eq(vec!["-value1", "-value2"])
}

#[test]
fn end_of_options_with_wrapped_command() -> Result {
    // def --wrapped passes -- through to the underlying program (allows_unknown_args),
    // so -- itself must appear in the rest args.
    test()
        .run(
            "
            def --wrapped my_wrap [...args] { $args }
            my_wrap -- -flag value
        ",
        )
        .expect_value_eq(["--", "-flag", "value"])
}

#[test]
fn end_of_options_multiple_occurrences() -> Result {
    // First -- stops flag parsing; remaining -- is a literal string arg
    test()
        .run("echo arg1 -- -arg2 -- -arg3")
        .expect_value_eq(vec!["arg1", "-arg2", "--", "-arg3"])
}

#[test]
fn end_of_options_with_spread_operator() -> Result {
    // Spread operator still works after --
    test()
        .run(
            r#"
            def my_cmd [...rest] { $rest }
            my_cmd -- ...["-a" "-b"]
        "#,
        )
        .expect_value_eq(["-a", "-b"])
}

#[test]
fn end_of_options_with_unknown_args() -> Result {
    // --unknown after -- is a positional, but my_cmd has no positionals -> parse error
    let code = r#"
        def my_cmd [--flag] {
            if ($flag == null) { "no flag" } else { $flag }
        }
        my_cmd -- --unknown
    "#;
    test().run(code).expect_parse_error()?;
    Ok(())
}

#[test]
fn custom_command_short_flags_before_end_of_options() -> Result {
    // -f and --long are boolean switches; value1 becomes positional x
    test()
        .run(
            "
            def my_cmd [-f --long x y z] { [$x $y $z] }
            my_cmd -f --long value1 -- -value2 -value3
        ",
        )
        .expect_value_eq(vec!["value1", "-value2", "-value3"])
}

#[test]
fn end_of_options_preserves_special_chars() -> Result {
    // -n and -e are passed as literal strings, not flags
    let vals: Vec<String> = test().run(r#"echo -- "-n" "-e" "test\n""#)?;
    assert!(vals.contains(&"-n".to_string()));
    assert!(vals.contains(&"-e".to_string()));
    Ok(())
}

#[test]
fn end_of_options_with_equals_syntax() -> Result {
    test()
        .run(
            "
            def my_cmd [--flag: string, y] { [$flag $y] }
            my_cmd --flag=value1 -- -value2
        ",
        )
        .expect_value_eq(vec!["value1", "-value2"])
}

#[test]
fn end_of_options_no_args_after() -> Result {
    test()
        .run(
            r#"
            def my_cmd [x?] { if ($x == null) { "empty" } else { $x } }
            my_cmd --
        "#,
        )
        .expect_value_eq("empty")
}

#[test]
fn external_command_with_end_of_options() -> Result {
    // External commands receive -- and following args verbatim
    let out: String = test().inherit_path().run("^echo -- -n hello")?;
    assert!(out.contains("--"), "expected '--' in output, got: {out}");
    assert!(out.contains("-n"), "expected '-n' in output, got: {out}");
    Ok(())
}

#[test]
fn custom_command_rest_param_with_end_of_options() -> Result {
    test()
        .run(
            "
            def collect_args [...args] { $args | length }
            collect_args -- -a -b -c
        ",
        )
        .expect_value_eq(3i64)
}

#[test]
fn end_of_options_with_negative_numbers() -> Result {
    // -5 after -- is treated as a positional integer literal, not a flag
    test()
        .run(
            "
            def add_nums [x y] { $x + $y }
            add_nums -- -5 10
        ",
        )
        .expect_value_eq(5i64)
}

#[test]
fn custom_command_mixed_positionals_and_flags_with_end_of_options() -> Result {
    test()
        .run(
            "
            def complex [x --flag: string y z] { [$x $flag $y $z] }
            complex first --flag flagval -- -second -third
        ",
        )
        .expect_value_eq(vec!["first", "flagval", "-second", "-third"])
}
