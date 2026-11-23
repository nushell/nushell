use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn capture_errors_works() -> Result {
    test()
        .run("do -c {$env.use}")
        .expect_error_code_eq("nu::shell::env_variable_not_found")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external() -> Result {
    let err = test().run("do -c {fail}").expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external_with_pipeline() -> Result {
    let err = test()
        .run("do -c {fail} | echo `text`")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external_with_semicolon() -> Result {
    let err = test()
        .run("do -c {fail}; echo `text`")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn do_with_semicolon_break_on_failed_external() -> Result {
    test()
        .run("do { fail }; `text`")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_error_should_work_for_external_command() -> Result {
    test()
        .run("do -i { fail 1 }; echo post")
        .expect_value_eq("post")
}

#[test]
fn ignore_error_works_with_list_stream() -> Result {
    let _: Value = test().run(r#"do -i { ["a", null, "b"] | ansi strip }"#)?;
    Ok(())
}

#[test]
fn run_closure_with_do_using() -> Result {
    test()
        .run("let x = {let var = 3; $var}; do $x")
        .expect_value_eq(3)
}

#[test]
fn required_argument_type_checked() -> Result {
    test()
        .run("do {|x: string| $x} 4")
        .expect_error_code_eq("nu::shell::cant_convert")
}

#[test]
fn optional_argument_type_checked() -> Result {
    test()
        .run("do {|x?: string| $x} 4")
        .expect_error_code_eq("nu::shell::cant_convert")
}

#[rstest]
#[case::record("do {|{$a, b: $c}| [$a $c] } {a: 1, b: 2}", test_value!([1, 2]))]
#[case::list("do {|[$x, ..$rest]| [$x $rest] } [1 2 3]", test_value!([1, [2, 3]]))]
#[case::after_plain_param("do {|n, {$a}| $n + $a } 1 {a: 2}", test_value!(3))]
#[case::each_row("[[a]; [1] [2]] | each {|{$a}| $a * 10 }", test_value!([10, 20]))]
#[case::zip_pairs("[1 2] | zip [3 4] | each {|[$x, $y]| $x * $y }", test_value!([3, 8]))]
#[case::reduce("[[a]; [1] [2]] | reduce --fold 0 {|{$a}, acc| $acc + $a }", test_value!(3))]
#[case::annotated("do {|{$a}: record<a: int>| $a } {a: 1, b: 2}", test_value!(1))]
#[case::annotated_glob(
    "do {|{$p}: record<p: glob>| $p | describe } {p: '*.md'}",
    test_value!("glob")
)]
#[case::default_value("do {|{$a} = {a: 3}| $a }", test_value!(3))]
#[case::optional_param("do {|{$a}?| $a }", test_value!(()))]
#[case::captured("let c = 10; do {|{$a}| $a + $c } {a: 1}", test_value!(11))]
#[case::nested_closure("do (do {|[$p, $q]| {|| $p + $q } } [1 2])", test_value!(3))]
fn closure_pattern_parameter(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn closure_pattern_parameter_mismatch() -> Result {
    let err = test().run("do {|{$a}| $a } 5").expect_shell_error()?;
    assert_matches!(err, ShellError::PatternMismatch { msg, .. } if msg == "expected a record, got int");
    Ok(())
}

#[test]
fn closure_pattern_parameter_annotation_mismatch() -> Result {
    test()
        .run("do {|{$a}: int| $a } 5")
        .expect_error_code_eq("nu::parser::type_mismatch")
}
