use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn env_shorthand() -> TestResult {
    run_test("FOO=BAR if $false { 3 } else { 4 }", "4")
}

#[test]
fn subcommand() -> TestResult {
    run_test("def foo [] {}; def \"foo bar\" [] {3}; foo bar", "3")
}

#[test]
fn alias_1() -> TestResult {
    run_test("def foo [$x] { $x + 10 }; alias f = foo; f 100", "110")
}

#[test]
fn alias_2() -> TestResult {
    run_test(
        "def foo [$x $y] { $x + $y + 10 }; alias f = foo 33; f 100",
        "143",
    )
}

#[test]
fn alias_2_multi_word() -> TestResult {
    run_test(
        r#"def "foo bar" [$x $y] { $x + $y + 10 }; alias f = foo bar 33; f 100"#,
        "143",
    )
}

#[test]
fn block_param1() -> TestResult {
    run_test("[3] | each { $it + 10 } | get 0", "13")
}

#[test]
fn block_param2() -> TestResult {
    run_test("[3] | each { |y| $y + 10 } | get 0", "13")
}

#[test]
fn block_param3_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { $it + 10 } | get 1", "12")
}

#[test]
fn block_param4_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { |y| $y + 10 } | get 2", "13")
}

#[test]
fn range_iteration1() -> TestResult {
    run_test("1..4 | each { |y| $y + 10 } | get 0", "11")
}

#[test]
fn range_iteration2() -> TestResult {
    run_test("4..1 | each { |y| $y + 100 } | get 3", "101")
}

#[test]
fn simple_value_iteration() -> TestResult {
    run_test("4 | each { $it + 10 }", "14")
}

#[test]
fn comment_multiline() -> TestResult {
    run_test(
        r#"def foo [] {
        let x = 1 + 2 # comment
        let y = 3 + 4 # another comment
        $x + $y
    }; foo"#,
        "10",
    )
}

#[test]
fn comment_skipping_1() -> TestResult {
    run_test(
        r#"let x = {
        y: 20
        # foo
    }; $x.y"#,
        "20",
    )
}

#[test]
fn comment_skipping_2() -> TestResult {
    run_test(
        r#"let x = {
        y: 20
        # foo
        z: 40
    }; $x.z"#,
        "40",
    )
}

#[test]
fn bad_var_name() -> TestResult {
    fail_test(r#"let $"foo bar" = 4"#, "can't contain")
}

#[test]
fn long_flag() -> TestResult {
    run_test(
        r#"([a, b, c] | each --numbered { if $it.index == 1 { 100 } else { 0 } }).1"#,
        "100",
    )
}
