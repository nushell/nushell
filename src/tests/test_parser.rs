use crate::tests::{fail_test, run_test, run_test_with_env, TestResult};
use std::collections::HashMap;

use super::run_test_contains;

#[test]
fn env_shorthand() -> TestResult {
    run_test("FOO=BAR if false { 3 } else { 4 }", "4")
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
fn ints_with_underscores() -> TestResult {
    run_test("1_0000_0000_0000 + 10", "1000000000010")
}

#[test]
fn floats_with_underscores() -> TestResult {
    run_test("3.1415_9265_3589_793 * 2", "6.283185307179586")
}

#[test]
fn bin_ints_with_underscores() -> TestResult {
    run_test("0b_10100_11101_10010", "21426")
}

#[test]
fn oct_ints_with_underscores() -> TestResult {
    run_test("0o2443_6442_7652_0044", "90422533333028")
}

#[test]
fn hex_ints_with_underscores() -> TestResult {
    run_test("0x68__9d__6a", "6856042")
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

#[ignore = "TODO: Allow alias to alias existing command with the same name"]
#[test]
fn alias_recursion() -> TestResult {
    run_test_contains(r#"alias ls = ls -a; ls"#, " ")
}

#[test]
fn block_param1() -> TestResult {
    run_test("[3] | each { |it| $it + 10 } | get 0", "13")
}

#[test]
fn block_param2() -> TestResult {
    run_test("[3] | each { |y| $y + 10 } | get 0", "13")
}

#[test]
fn block_param3_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { |it| $it + 10 } | get 1", "12")
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
    run_test("4 | each { |it| $it + 10 }", "14")
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
fn bad_var_name2() -> TestResult {
    fail_test(r#"let $foo-bar = 4"#, "valid variable")
}

#[test]
fn long_flag() -> TestResult {
    run_test(
        r#"([a, b, c] | enumerate | each --keep-empty { |e| if $e.index != 1 { 100 }}).1 | to nuon"#,
        "null",
    )
}

#[test]
fn let_not_statement() -> TestResult {
    fail_test(r#"let x = "hello" | str length"#, "used in pipeline")
}

#[test]
fn for_in_missing_var_name() -> TestResult {
    fail_test("for in", "missing")
}

#[test]
fn multiline_pipe_in_block() -> TestResult {
    run_test(
        r#"do {
            echo hello |
            str length
        }"#,
        "5",
    )
}

#[test]
fn bad_short_flag() -> TestResult {
    fail_test(r#"def foo3 [-l?:int] { $l }"#, "short flag")
}

#[test]
fn quotes_with_equals() -> TestResult {
    run_test(
        r#"let query_prefix = "https://api.github.com/search/issues?q=repo:nushell/"; $query_prefix"#,
        "https://api.github.com/search/issues?q=repo:nushell/",
    )
}

#[test]
fn string_interp_with_equals() -> TestResult {
    run_test(
        r#"let query_prefix = $"https://api.github.com/search/issues?q=repo:nushell/"; $query_prefix"#,
        "https://api.github.com/search/issues?q=repo:nushell/",
    )
}

#[test]
fn recursive_parse() -> TestResult {
    run_test(r#"def c [] { c }; echo done"#, "done")
}

#[test]
fn commands_have_usage() -> TestResult {
    run_test_contains(
        r#"
    # This is a test
    #
    # To see if I have cool usage
    def foo [] {}
    help foo"#,
        "cool usage",
    )
}

#[test]
fn equals_separates_long_flag() -> TestResult {
    run_test(
        r#"'nushell' | fill --alignment right --width=10 --character='-'"#,
        "---nushell",
    )
}

#[test]
fn let_env_expressions() -> TestResult {
    let env = HashMap::from([("VENV_OLD_PATH", "Foobar"), ("Path", "Quux")]);
    run_test_with_env(
        r#"let-env Path = if ($env | columns | "VENV_OLD_PATH" in $in) { $env.VENV_OLD_PATH } else { $env.Path }; echo $env.Path"#,
        "Foobar",
        &env,
    )
}

#[test]
fn string_interpolation_paren_test() -> TestResult {
    run_test(r#"$"('(')(')')""#, "()")
}

#[test]
fn string_interpolation_paren_test2() -> TestResult {
    run_test(r#"$"('(')test(')')""#, "(test)")
}

#[test]
fn string_interpolation_paren_test3() -> TestResult {
    run_test(r#"$"('(')("test")test(')')""#, "(testtest)")
}

#[test]
fn string_interpolation_escaping() -> TestResult {
    run_test(r#"$"hello\nworld" | lines | length"#, "2")
}

#[test]
fn capture_multiple_commands() -> TestResult {
    run_test(
        r#"
let CONST_A = 'Hello'

def 'say-hi' [] {
    echo (call-me)
}

def 'call-me' [] {
    echo $CONST_A
}

[(say-hi) (call-me)] | str join

    "#,
        "HelloHello",
    )
}

#[test]
fn capture_multiple_commands2() -> TestResult {
    run_test(
        r#"
let CONST_A = 'Hello'

def 'call-me' [] {
    echo $CONST_A
}

def 'say-hi' [] {
    echo (call-me)
}

[(say-hi) (call-me)] | str join

    "#,
        "HelloHello",
    )
}

#[test]
fn capture_multiple_commands3() -> TestResult {
    run_test(
        r#"
let CONST_A = 'Hello'

def 'say-hi' [] {
    echo (call-me)
}

def 'call-me' [] {
    echo $CONST_A
}

[(call-me) (say-hi)] | str join

    "#,
        "HelloHello",
    )
}

#[test]
fn capture_multiple_commands4() -> TestResult {
    run_test(
        r#"
let CONST_A = 'Hello'

def 'call-me' [] {
    echo $CONST_A
}

def 'say-hi' [] {
    echo (call-me)
}

[(call-me) (say-hi)] | str join

    "#,
        "HelloHello",
    )
}

#[test]
fn capture_row_condition() -> TestResult {
    run_test(
        r#"let name = "foo"; [foo] | where $'($name)' =~ $it | str join"#,
        "foo",
    )
}

#[test]
fn starts_with_operator_succeeds() -> TestResult {
    run_test(
        r#"[Moe Larry Curly] | where $it starts-with L | str join"#,
        "Larry",
    )
}

#[test]
fn ends_with_operator_succeeds() -> TestResult {
    run_test(
        r#"[Moe Larry Curly] | where $it ends-with ly | str join"#,
        "Curly",
    )
}

#[test]
fn proper_missing_param() -> TestResult {
    fail_test(r#"def foo [x y z w] { }; foo a b c"#, "missing w")
}

#[test]
fn block_arity_check1() -> TestResult {
    fail_test(
        r#"ls | each { |x, y, z| 1}"#,
        "expected 2 closure parameters",
    )
}

// deprecating former support for escapes like `/uNNNN`, dropping test.
#[test]
fn string_escape_unicode_extended() -> TestResult {
    run_test(r#""\u{015B}\u{1f10b}""#, "ś🄋")
}

#[test]
fn string_escape_interpolation() -> TestResult {
    run_test(r#"$"\u{015B}(char hamburger)abc""#, "ś≡abc")
}

#[test]
fn string_escape_interpolation2() -> TestResult {
    run_test(r#"$"2 + 2 is \(2 + 2)""#, "2 + 2 is (2 + 2)")
}

#[test]
fn proper_rest_types() -> TestResult {
    run_test(
        r#"def foo [--verbose(-v): bool, # my test flag
                   ...rest: int # my rest comment
                ] { if $verbose { print "verbose!" } else { print "not verbose!" } }; foo"#,
        "not verbose!",
    )
}

#[test]
fn single_value_row_condition() -> TestResult {
    run_test(
        r#"[[a, b]; [true, false], [true, true]] | where a | length"#,
        "2",
    )
}

#[test]
fn unary_not_1() -> TestResult {
    run_test(r#"not false"#, "true")
}

#[test]
fn unary_not_2() -> TestResult {
    run_test(r#"not (false)"#, "true")
}

#[test]
fn unary_not_3() -> TestResult {
    run_test(r#"(not false)"#, "true")
}

#[test]
fn unary_not_4() -> TestResult {
    run_test(r#"if not false { "hello" } else { "world" }"#, "hello")
}

#[test]
fn unary_not_5() -> TestResult {
    run_test(
        r#"if not not not not false { "hello" } else { "world" }"#,
        "world",
    )
}

#[test]
fn unary_not_6() -> TestResult {
    run_test(
        r#"[[name, present]; [abc, true], [def, false]] | where not present | get name.0"#,
        "def",
    )
}

#[test]
fn date_literal() -> TestResult {
    run_test(r#"2022-09-10 | date to-record | get day"#, "10")
}

#[test]
fn and_and_or() -> TestResult {
    run_test(r#"true and false or true"#, "true")
}

#[test]
fn and_and_xor() -> TestResult {
    // Assumes the precedence NOT > AND > XOR > OR
    run_test(r#"true and true xor true and false"#, "true")
}

#[test]
fn or_and_xor() -> TestResult {
    // Assumes the precedence NOT > AND > XOR > OR
    run_test(r#"true or false xor true or false"#, "true")
}

#[test]
fn unbalanced_delimiter() -> TestResult {
    fail_test(r#"{a:{b:5}}}"#, "unbalanced { and }")
}

#[test]
fn unbalanced_delimiter2() -> TestResult {
    fail_test(r#"{}#.}"#, "unbalanced { and }")
}

#[test]
fn unbalanced_delimiter3() -> TestResult {
    fail_test(r#"{"#, "Unexpected end of code")
}

#[test]
fn unbalanced_delimiter4() -> TestResult {
    fail_test(r#"}"#, "unbalanced { and }")
}

#[test]
fn register_with_string_literal() -> TestResult {
    fail_test(r#"register 'nu-plugin-math'"#, "File not found")
}

#[test]
fn register_with_string_constant() -> TestResult {
    let input = "\
const file = 'nu-plugin-math'
register $file
";
    // should not fail with `not a constant`
    fail_test(input, "File not found")
}

#[test]
fn register_with_string_variable() -> TestResult {
    let input = "\
let file = 'nu-plugin-math'
register $file
";
    fail_test(input, "Value is not a parse-time constant")
}

#[test]
fn register_with_non_string_constant() -> TestResult {
    let input = "\
const file = 6
register $file
";
    fail_test(input, "expected string, found int")
}

#[test]
fn extern_errors_with_no_space_between_params_and_name_1() -> TestResult {
    fail_test("extern cmd[]", "expected space")
}

#[test]
fn extern_errors_with_no_space_between_params_and_name_2() -> TestResult {
    fail_test("extern cmd(--flag)", "expected space")
}
