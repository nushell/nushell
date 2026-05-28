/// Tests for POSIX `--` end-of-options delimiter support
///
/// This test file validates that `--` stops all flag parsing and treats
/// all following arguments as operands, even if they start with `-` or `--`.
/// This aligns with POSIX Guideline 10:
/// https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html#tag_12_02
use nu_test_support::nu;

#[test]
fn echo_with_end_of_options_stops_flag_parsing() {
    let actual = nu!("echo -- -n hello | str join \" \"");
    assert_eq!(actual.out, "-n hello");
}

#[test]
fn echo_with_end_of_options_multiple_dashes() {
    let actual = nu!("echo -- --foo -bar baz | str join \" \"");
    assert_eq!(actual.out, "--foo -bar baz");
}

#[test]
fn echo_with_end_of_options_alone() {
    let actual = nu!("echo --");
    assert_eq!(actual.out, "");
}

#[test]
fn echo_before_and_after_end_of_options() {
    let actual = nu!("echo arg1 -- -arg2 | str join \" \"");
    assert_eq!(actual.out, "arg1 -arg2");
}

#[test]
fn custom_command_with_end_of_options_and_positional() {
    let actual = nu!("
def my_cmd [x y] { [$x $y] }
my_cmd -- -value1 -value2
");
    assert!(actual.out.contains("-value1"));
    assert!(actual.out.contains("-value2"));
}

#[test]
fn custom_command_with_end_of_options_and_rest() {
    let actual = nu!(r#"
def my_cmd [x ...rest] { [$x ($rest | str join " ")] }
my_cmd first -- -second -third
"#);
    assert!(actual.out.contains("first"));
    assert!(actual.out.contains("-second"));
    assert!(actual.out.contains("-third"));
}

#[test]
fn custom_command_with_flags_before_end_of_options() {
    let actual = nu!("
def my_cmd [--flag x y z] { [$flag $x $y $z] }
my_cmd --flag value1 -- -value2 -value3
");
    assert!(actual.out.contains("value1"));
    assert!(actual.out.contains("-value2"));
    assert!(actual.out.contains("-value3"));
}

#[test]
fn custom_command_with_optional_positional_and_end_of_options() {
    let actual = nu!("
def my_cmd [x y?] { if ($y == null) { $x } else { [$x $y] } }
my_cmd -- -value1 -value2
");
    assert!(actual.out.contains("-value1"));
    assert!(actual.out.contains("-value2"));
}

#[test]
fn end_of_options_with_wrapped_command() {
    let actual = nu!(r#"
def --wrapped my_wrap [...args] { $args | str join " " }
my_wrap -- -flag value
"#);
    // In wrapped mode, -- should still be recognized and consumed
    assert!(actual.out.contains("-flag"));
    assert!(actual.out.contains("value"));
    assert!(!actual.out.contains("--")); // -- should be consumed
}

#[test]
fn end_of_options_multiple_occurrences() {
    let actual = nu!("echo arg1 -- -arg2 -- -arg3");
    // First -- stops parsing; remaining -- is literal
    assert!(actual.out.contains("arg1"));
    assert!(actual.out.contains("-arg2"));
    assert!(actual.out.contains("--")); // Second -- is a literal arg
    assert!(actual.out.contains("-arg3"));
}

#[test]
fn end_of_options_with_spread_operator() {
    let actual = nu!(r#"
def my_cmd [...rest] { $rest | str join " " }
my_cmd -- ...["-a" "-b"]
"#);
    // The spread should still work after --
    assert!(actual.out.contains("-a"));
    assert!(actual.out.contains("-b"));
}

#[test]
fn end_of_options_with_unknown_args() {
    let actual = nu!(r#"
def my_cmd [--flag] { 
    if ($flag == null) { "no flag" } else { $flag }
}
my_cmd -- --unknown
"#);
    // --unknown after -- is treated as a positional, but my_cmd has no positionals -> error
    assert!(
        actual.err.contains("extra") || actual.err.contains("positional"),
        "Expected error about extra positional, got: {}",
        actual.err
    );
}

#[test]
fn custom_command_short_flags_before_end_of_options() {
    let actual = nu!("
def my_cmd [-f --long x y z] { [$f $long $x $y $z] }
my_cmd -f --long value1 -- -value2 -value3
");
    assert!(actual.out.contains("value1"));
    assert!(actual.out.contains("-value2"));
    assert!(actual.out.contains("-value3"));
}

#[test]
fn end_of_options_preserves_special_chars() {
    let actual = nu!(r#"echo -- "-n" "-e" "test\n""#);
    assert!(actual.out.contains("-n"));
    assert!(actual.out.contains("-e"));
    // The \n should be literal, not interpreted as a newline by echo (since -n is not a flag)
}

#[test]
fn end_of_options_with_equals_syntax() {
    let actual = nu!("
def my_cmd [--flag: string, y] { [$flag $y] }
my_cmd --flag=value1 -- -value2
");
    assert!(actual.out.contains("value1"));
    assert!(actual.out.contains("-value2"));
}

#[test]
fn end_of_options_no_args_after() {
    let actual = nu!(r#"
def my_cmd [x?] { if ($x == null) { "empty" } else { $x } }
my_cmd --
"#);
    assert!(actual.out.contains("empty"));
}

#[test]
fn external_command_with_end_of_options() {
    // External commands should pass -- through as-is (already working)
    let actual = nu!("^echo -- -n hello");
    assert!(actual.out.contains("--"));
    assert!(actual.out.contains("-n"));
}

#[test]
fn test_command_with_end_of_options() {
    // `test` builtin with --
    let actual = nu!(r#"test -- -z"""#);
    // Should not error; -z should be treated as a string operand, not a flag
    assert!(!actual.err.contains("unknown flag"));
}

#[test]
fn custom_command_rest_param_with_end_of_options() {
    let actual = nu!("
def collect_args [...args] { $args | length }
collect_args -- -a -b -c
");
    assert_eq!(actual.out, "3");
}

#[test]
fn end_of_options_with_negative_numbers() {
    let actual = nu!("
def add_nums [x y] { $x + $y }
add_nums -- -5 10
");
    // -5 should be treated as positional arg, not a flag
    assert_eq!(actual.out, "5");
}

#[test]
fn custom_command_mixed_positionals_and_flags_with_end_of_options() {
    let actual = nu!("
def complex [x --flag: string y z] { [$x $flag $y $z] }
complex first --flag flagval -- -second -third
");
    assert!(actual.out.contains("first"));
    assert!(actual.out.contains("flagval"));
    assert!(actual.out.contains("-second"));
    assert!(actual.out.contains("-third"));
}
