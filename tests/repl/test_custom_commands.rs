use crate::repl::tests::{TestResult, fail_test, run_test, run_test_contains};
use nu_test_support::nu;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[test]
fn no_scope_leak1() -> TestResult {
    fail_test(
        "if false { let $x = 10 } else { let $x = 20 }; $x",
        "Variable not found",
    )
}

#[test]
fn no_scope_leak2() -> TestResult {
    fail_test(
        "def foo [] { $x }; def bar [] { let $x = 10; foo }; bar",
        "Variable not found",
    )
}

#[test]
fn no_scope_leak3() -> TestResult {
    run_test(
        "def foo [$x] { $x }; def bar [] { let $x = 10; foo 20}; bar",
        "20",
    )
}

#[test]
fn no_scope_leak4() -> TestResult {
    run_test(
        "def foo [$x] { $x }; def bar [] { let $x = 10; (foo 20) + $x}; bar",
        "30",
    )
}

#[test]
fn custom_rest_var() -> TestResult {
    run_test("def foo [...x] { $x.0 + $x.1 }; foo 10 80", "90")
}

#[test]
fn def_twice_should_fail() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; def foo [] { "bar" }"#,
        "defined more than once",
    )
}

#[test]
fn missing_parameters() -> TestResult {
    fail_test(r#"def foo {}"#, "expected [ or (")
}

#[test]
fn flag_param_value() -> TestResult {
    run_test(
        r#"def foo [--bob: int] { $bob + 100 }; foo --bob 55"#,
        "155",
    )
}

#[test]
fn do_rest_args() -> TestResult {
    run_test(r#"(do { |...rest| $rest } 1 2).1 + 10"#, "12")
}

#[test]
fn custom_switch1() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { if ($dry_run) { "foo" } else { "bar" } }; florb --dry-run"#,
        "foo",
    )
}

#[rstest]
fn custom_flag_with_type_checking(
    #[values(
        ("int", "\"3\""),
        ("int", "null"),
        ("record<i: int>", "{i: \"\"}"),
        ("list<int>", "[\"\"]")
    )]
    type_sig_value: (&str, &str),
    #[values("--dry-run", "-d")] flag: &str,
) -> TestResult {
    let (type_sig, value) = type_sig_value;

    fail_test(
        &format!("def florb [{flag}: {type_sig}] {{}}; let y = {value}; florb {flag} $y"),
        "type_mismatch",
    )
}

#[test]
fn custom_switch2() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { if ($dry_run) { "foo" } else { "bar" } }; florb"#,
        "bar",
    )
}

#[test]
fn custom_switch3() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { $dry_run }; florb --dry-run=false"#,
        "false",
    )
}

#[test]
fn custom_switch4() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { $dry_run }; florb --dry-run=true"#,
        "true",
    )
}

#[test]
fn custom_switch5() -> TestResult {
    run_test(r#"def florb [ --dry-run ] { $dry_run }; florb"#, "false")
}

#[test]
fn custom_switch6() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { $dry_run }; florb --dry-run"#,
        "true",
    )
}

#[test]
fn custom_flag1() -> TestResult {
    run_test(
        r#"def florb [
            --age: int = 0
            --name = "foobar"
        ] {
            ($age | into string) + $name
        }
        florb"#,
        "0foobar",
    )
}

#[test]
fn custom_flag2() -> TestResult {
    run_test(
        r#"def florb [
            --age: int
            --name = "foobar"
        ] {
            ($age | into string) + $name
        }
        florb --age 3"#,
        "3foobar",
    )
}

#[test]
fn deprecated_boolean_flag() {
    let actual = nu!(r#"def florb [--dry-run: bool, --another-flag] { "aaa" };  florb"#);
    assert!(actual.err.contains("not allowed"));
}

#[test]
fn simple_var_closing() -> TestResult {
    run_test("let $x = 10; def foo [] { $x }; foo", "10")
}

#[test]
fn predecl_check() -> TestResult {
    run_test("def bob [] { sam }; def sam [] { 3 }; bob", "3")
}

#[test]
fn def_with_no_dollar() -> TestResult {
    run_test("def bob [x] { $x + 3 }; bob 4", "7")
}

#[test]
fn allow_missing_optional_params() -> TestResult {
    run_test(
        "def foo [x?:int] { if $x != null { $x + 10 } else { 5 } }; foo",
        "5",
    )
}

#[test]
fn help_present_in_def() -> TestResult {
    run_test_contains(
        "def foo [] {}; help foo;",
        "Display the help message for this command",
    )
}

#[test]
fn help_not_present_in_extern() -> TestResult {
    run_test(
        r#"
            module test {export extern "git fetch" []};
            use test `git fetch`;
            help git fetch | find help | to text | ansi strip
        "#,
        "",
    )
}

#[test]
fn override_table() -> TestResult {
    run_test(r#"def table [-e] { "hi" }; table"#, "hi")
}

#[test]
fn override_table_eval_file() {
    let actual = nu!(r#"def table [-e] { "hi" }; table"#);
    assert_eq!(actual.out, "hi");
}

// This test is disabled on Windows because they cause a stack overflow in CI (but not locally!).
// For reasons we don't understand, the Windows CI runners are prone to stack overflow.
// TODO: investigate so we can enable on Windows
#[cfg(not(target_os = "windows"))]
#[test]
fn infinite_recursion_does_not_panic() {
    let actual = nu!(r#"
            def bang [] { bang }; bang
        "#);
    assert!(actual.err.contains("Recursion limit (50) reached"));
}

// This test is disabled on Windows because they cause a stack overflow in CI (but not locally!).
// For reasons we don't understand, the Windows CI runners are prone to stack overflow.
// TODO: investigate so we can enable on Windows
#[cfg(not(target_os = "windows"))]
#[test]
fn infinite_mutual_recursion_does_not_panic() {
    let actual = nu!(r#"
            def bang [] { def boom [] { bang }; boom }; bang
        "#);
    assert!(actual.err.contains("Recursion limit (50) reached"));
}

#[test]
fn type_check_for_during_eval() -> TestResult {
    fail_test(
        r#"def spam [foo: string] { $foo | describe }; def outer [--foo: string] { spam $foo }; outer"#,
        "can't convert nothing to string",
    )
}
#[test]
fn type_check_for_during_eval2() -> TestResult {
    fail_test(
        r#"def spam [foo: string] { $foo | describe }; def outer [--foo: any] { spam $foo }; outer"#,
        "can't convert nothing to string",
    )
}

#[test]
fn empty_list_matches_list_type() -> TestResult {
    let _ = run_test(
        r#"def spam [foo: list<int>] { echo $foo }; spam [] | length"#,
        "0",
    );
    run_test(
        r#"def spam [foo: list<string>] { echo $foo }; spam [] | length"#,
        "0",
    )
}

#[test]
fn path_argument_dont_auto_expand_if_single_quoted() -> TestResult {
    run_test("def spam [foo: path] { echo $foo }; spam '~/aa'", "~/aa")
}

#[test]
fn path_argument_dont_auto_expand_if_double_quoted() -> TestResult {
    run_test(r#"def spam [foo: path] { echo $foo }; spam "~/aa""#, "~/aa")
}

#[test]
fn path_argument_dont_make_absolute_if_unquoted() -> TestResult {
    #[cfg(windows)]
    let expected = "..\\bar";
    #[cfg(not(windows))]
    let expected = "../bar";
    run_test(
        r#"def spam [foo: path] { echo $foo }; spam foo/.../bar"#,
        expected,
    )
}

#[test]
fn dont_allow_implicit_casting_between_glob_and_string() -> TestResult {
    let _ = fail_test(
        r#"def spam [foo: string] { echo $foo }; let f: glob = 'aa'; spam $f"#,
        "expected string, found glob",
    );
    run_test(
        r#"def spam [foo: glob] { echo $foo }; let f = 'aa'; spam $f"#,
        "aa",
    )
}

#[test]
fn allow_pass_negative_float() -> TestResult {
    run_test("def spam [val: float] { $val }; spam -1.4", "-1.4")?;
    run_test("def spam [val: float] { $val }; spam -2", "-2.0")
}
