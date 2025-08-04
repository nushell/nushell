use crate::repl::tests::{TestResult, fail_test, run_test, run_test_contains, run_test_with_env};
use nu_test_support::{nu, nu_repl_code};
use std::collections::HashMap;

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
fn range_ends_with_duration_suffix_variable_name() -> TestResult {
    run_test("let runs = 10; 1..$runs | math sum", "55")
}

#[test]
fn range_ends_with_filesize_suffix_variable_name() -> TestResult {
    run_test("let sizekb = 10; 1..$sizekb | math sum", "55")
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
fn comment_skipping_in_pipeline_1() -> TestResult {
    run_test(
        r#"[1,2,3] | #comment
        each { |$it| $it + 2 } | # foo
        math sum #bar"#,
        "12",
    )
}

#[test]
fn comment_skipping_in_pipeline_2() -> TestResult {
    run_test(
        r#"[1,2,3] #comment
        | #comment2
        each { |$it| $it + 2 } #foo
        | # bar
        math sum #baz"#,
        "12",
    )
}

#[test]
fn comment_skipping_in_pipeline_3() -> TestResult {
    run_test(
        r#"[1,2,3] | #comment
        #comment2
        each { |$it| $it + 2 } #foo
        | # bar
        #baz
        math sum #foobar"#,
        "12",
    )
}

#[test]
fn still_string_if_hashtag_is_middle_of_string() -> TestResult {
    run_test(r#"echo test#testing"#, "test#testing")
}

#[test]
fn non_comment_hashtag_in_comment_does_not_stop_comment() -> TestResult {
    run_test(r#"# command_bar_text: { fg: '#C4C9C6' },"#, "")
}

#[test]
fn non_comment_hashtag_in_comment_does_not_stop_comment_in_block() -> TestResult {
    run_test(
        r#"{
        explore: {
            # command_bar_text: { fg: '#C4C9C6' },
        }
    } | get explore | is-empty"#,
        "true",
    )
}

#[test]
fn still_string_if_hashtag_is_middle_of_string_inside_each() -> TestResult {
    run_test(
        r#"1..1 | each {echo test#testing } | get 0"#,
        "test#testing",
    )
}

#[test]
fn still_string_if_hashtag_is_middle_of_string_inside_each_also_with_dot() -> TestResult {
    run_test(r#"1..1 | each {echo '.#testing' } | get 0"#, ".#testing")
}

#[test]
fn bad_var_name() -> TestResult {
    fail_test(r#"let $"foo bar" = 4"#, "can't contain")
}

#[test]
fn bad_var_name2() -> TestResult {
    fail_test(r#"let $foo-bar = 4"#, "valid variable")?;
    fail_test(r#"foo-bar=4 true"#, "Command `foo-bar=4` not found")
}

#[test]
fn bad_var_name3() -> TestResult {
    fail_test(r#"let $=foo = 4"#, "valid variable")?;
    fail_test(r#"=foo=4 true"#, "Command `=foo=4` not found")
}

#[test]
fn assignment_with_no_var() -> TestResult {
    let cases = [
        "let = if $",
        "mut = if $",
        "const = if $",
        "let = 'foo' | $in; $x | describe",
        "mut = 'foo' | $in; $x | describe",
    ];

    let expecteds = [
        "missing var_name",
        "missing var_name",
        "missing const_name",
        "missing var_name",
        "missing var_name",
    ];

    for (case, expected) in std::iter::zip(cases, expecteds) {
        fail_test(case, expected)?;
    }

    Ok(())
}

#[test]
fn too_few_arguments() -> TestResult {
    // Test for https://github.com/nushell/nushell/issues/9072
    let cases = [
        "def a [b: bool, c: bool, d: float, e: float, f: float] {}; a true true 1 1",
        "def a [b: bool, c: bool, d: float, e: float, f: float, g: float] {}; a true true 1 1",
        "def a [b: bool, c: bool, d: float, e: float, f: float, g: float, h: float] {}; a true true 1 1",
    ];

    let expected = "missing f";

    for case in cases {
        fail_test(case, expected)?;
    }

    Ok(())
}

#[test]
fn long_flag() -> TestResult {
    run_test(
        r#"([a, b, c] | enumerate | each --keep-empty { |e| if $e.index != 1 { 100 }}).1 | to nuon"#,
        "null",
    )
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
fn raw_string_with_equals() -> TestResult {
    run_test(
        r#"let query_prefix = r#'https://api.github.com/search/issues?q=repo:nushell/'#; $query_prefix"#,
        "https://api.github.com/search/issues?q=repo:nushell/",
    )
}

#[test]
fn raw_string_with_hashtag() -> TestResult {
    run_test(r#"r##' one # two '##"#, "one # two")
}

#[test]
fn list_quotes_with_equals() -> TestResult {
    run_test(
        r#"["https://api.github.com/search/issues?q=repo:nushell/"] | get 0"#,
        "https://api.github.com/search/issues?q=repo:nushell/",
    )
}

#[test]
fn record_quotes_with_equals() -> TestResult {
    run_test(r#"{"a=":b} | get a="#, "b")?;
    run_test(r#"{"=a":b} | get =a"#, "b")?;

    run_test(r#"{a:"=b"} | get a"#, "=b")?;
    run_test(r#"{a:"b="} | get a"#, "b=")?;

    run_test(r#"{a:b,"=c":d} | get =c"#, "d")?;
    run_test(r#"{a:b,"c=":d} | get c="#, "d")
}

#[test]
fn recursive_parse() -> TestResult {
    run_test(r#"def c [] { c }; echo done"#, "done")
}

#[test]
fn commands_have_description() -> TestResult {
    run_test_contains(
        r#"
    # This is a test
    #
    # To see if I have cool description
    def foo [] {}
    help foo"#,
        "cool description",
    )
}

#[test]
fn commands_from_crlf_source_have_short_description() -> TestResult {
    run_test_contains(
        "# This is a test\r\n#\r\n# To see if I have cool description\r\ndef foo [] {}\r\nscope commands | where name == foo | get description.0",
        "This is a test",
    )
}

#[test]
fn commands_from_crlf_source_have_extra_description() -> TestResult {
    run_test_contains(
        "# This is a test\r\n#\r\n# To see if I have cool description\r\ndef foo [] {}\r\nscope commands | where name == foo | get extra_description.0",
        "To see if I have cool description",
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
fn assign_expressions() -> TestResult {
    let env = HashMap::from([("VENV_OLD_PATH", "Foobar"), ("Path", "Quux")]);
    run_test_with_env(
        r#"$env.Path = (if ($env | columns | "VENV_OLD_PATH" in $in) { $env.VENV_OLD_PATH } else { $env.Path }); echo $env.Path"#,
        "Foobar",
        &env,
    )
}

#[test]
fn assign_takes_pipeline() -> TestResult {
    run_test(
        r#"mut foo = 'bar'; $foo = $foo | str upcase | str reverse; $foo"#,
        "RAB",
    )
}

#[test]
fn append_assign_takes_pipeline() -> TestResult {
    run_test(
        r#"mut foo = 'bar'; $foo ++= $foo | str upcase; $foo"#,
        "barBAR",
    )
}

#[test]
fn assign_bare_external_fails() {
    let result = nu!("$env.FOO = nu --testbin cococo");
    assert!(!result.status.success());
    assert!(result.err.contains("must be explicit"));
}

#[test]
fn assign_bare_external_with_caret() {
    let result = nu!("$env.FOO = ^nu --testbin cococo");
    assert!(result.status.success());
}

#[test]
fn assign_backtick_quoted_external_fails() {
    let result = nu!("$env.FOO = `nu` --testbin cococo");
    assert!(!result.status.success());
    assert!(result.err.contains("must be explicit"));
}

#[test]
fn assign_backtick_quoted_external_with_caret() {
    let result = nu!("$env.FOO = ^`nu` --testbin cococo");
    assert!(result.status.success());
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
    fail_test(r#"ls | each { |x, y| 1}"#, "expected 1 closure parameter")
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
        r#"def foo [--verbose(-v), # my test flag
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
fn row_condition_non_boolean() -> TestResult {
    fail_test(r#"[1 2 3] | where 1"#, "expected bool")
}

#[test]
fn performance_nested_lists() -> TestResult {
    // Parser used to be exponential on deeply nested lists
    // TODO: Add a timeout
    fail_test(r#"[[[[[[[[[[[[[[[[[[[[[[[[[[[["#, "Unexpected end of code")
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
fn comment_in_multiple_pipelines() -> TestResult {
    run_test(
        r#"[[name, present]; [abc, true], [def, false]]
        # | where not present
        | get name.0"#,
        "abc",
    )
}

#[test]
fn date_literal() -> TestResult {
    run_test(r#"2022-09-10 | into record | get day"#, "10")
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
fn unbalanced_parens1() -> TestResult {
    fail_test(r#")"#, "unbalanced ( and )")
}

#[test]
fn unbalanced_parens2() -> TestResult {
    fail_test(r#"("("))"#, "unbalanced ( and )")
}

#[test]
fn plugin_use_with_string_literal() -> TestResult {
    fail_test(
        r#"plugin use 'nu-plugin-math'"#,
        "Plugin registry file not set",
    )
}

#[test]
fn plugin_use_with_string_constant() -> TestResult {
    let input = "\
const file = 'nu-plugin-math'
plugin use $file
";
    // should not fail with `not a constant`
    fail_test(input, "Plugin registry file not set")
}

#[test]
fn plugin_use_with_string_variable() -> TestResult {
    let input = "\
let file = 'nu-plugin-math'
plugin use $file
";
    fail_test(input, "Value is not a parse-time constant")
}

#[test]
fn plugin_use_with_non_string_constant() -> TestResult {
    let input = "\
const file = 6
plugin use $file
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

#[test]
fn duration_with_underscores_1() -> TestResult {
    run_test("420_min", "7hr")
}

#[test]
fn duration_with_underscores_2() -> TestResult {
    run_test("1_000_000sec", "1wk 4day 13hr 46min 40sec")
}

#[test]
fn duration_with_underscores_3() -> TestResult {
    fail_test("1_000_d_ay", "Command `1_000_d_ay` not found")
}

#[test]
fn duration_with_faulty_number() -> TestResult {
    fail_test("sleep 4-ms", "duration value must be a number")
}

#[test]
fn filesize_with_underscores_1() -> TestResult {
    run_test("420_MB", "420.0 MB")
}

#[test]
fn filesize_with_underscores_2() -> TestResult {
    run_test("1_000_000B", "1.0 MB")
}

#[test]
fn filesize_with_underscores_3() -> TestResult {
    fail_test("42m_b", "Command `42m_b` not found")
}

#[test]
fn filesize_is_not_hex() -> TestResult {
    run_test("0x42b", "1067")
}

#[test]
fn let_variable_type_mismatch() -> TestResult {
    fail_test(r#"let x: int = "foo""#, "expected int, found string")
}

#[test]
fn let_variable_disallows_completer() -> TestResult {
    fail_test(
        r#"let x: int@completer = 42"#,
        "Unexpected custom completer",
    )
}

#[test]
fn def_with_input_output() -> TestResult {
    run_test(r#"def foo []: nothing -> int { 3 }; foo"#, "3")
}

#[test]
fn def_with_input_output_with_line_breaks() -> TestResult {
    run_test(
        r#"def foo []: [
          nothing -> int
        ] { 3 }; foo"#,
        "3",
    )
}

#[test]
fn def_with_multi_input_output_with_line_breaks() -> TestResult {
    run_test(
        r#"def foo []: [
          nothing -> int
          string -> int
        ] { 3 }; foo"#,
        "3",
    )
}

#[test]
fn def_with_multi_input_output_without_commas() -> TestResult {
    run_test(
        r#"def foo []: [nothing -> int string -> int] { 3 }; foo"#,
        "3",
    )
}

#[test]
fn def_with_multi_input_output_called_with_first_sig() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { 3 }; 10 | foo"#,
        "3",
    )
}

#[test]
fn def_with_multi_input_output_called_with_second_sig() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { 3 }; "bob" | foo"#,
        "3",
    )
}

#[test]
fn def_with_input_output_mismatch_1() -> TestResult {
    fail_test(
        r#"def foo []: [int -> int, string -> int] { 3 }; foo"#,
        "command doesn't support",
    )
}

#[test]
fn def_with_input_output_mismatch_2() -> TestResult {
    fail_test(
        r#"def foo []: [int -> int, string -> int] { 3 }; {x: 2} | foo"#,
        "command doesn't support",
    )
}

#[test]
fn def_with_input_output_broken_1() -> TestResult {
    fail_test(r#"def foo []: int { 3 }"#, "expected arrow")
}

#[test]
fn def_with_input_output_broken_2() -> TestResult {
    fail_test(r#"def foo []: int -> { 3 }"#, "expected type")
}

#[test]
fn def_with_input_output_broken_3() -> TestResult {
    fail_test(
        r#"def foo []: int -> int@completer {}"#,
        "Unexpected custom completer",
    )
}

#[test]
fn def_with_input_output_broken_4() -> TestResult {
    fail_test(
        r#"def foo []: int -> list<int@completer> {}"#,
        "Unexpected custom completer",
    )
}

#[test]
fn def_with_in_var_let_1() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { let x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; "100" | foo"#,
        "4",
    )
}

#[test]
fn def_with_in_var_let_2() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { let x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; 100 | foo"#,
        "3",
    )
}

#[test]
fn def_with_in_var_mut_1() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { mut x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; "100" | foo"#,
        "4",
    )
}

#[test]
fn def_with_in_var_mut_2() -> TestResult {
    run_test(
        r#"def foo []: [int -> int, string -> int] { mut x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; 100 | foo"#,
        "3",
    )
}

#[test]
fn properly_nest_captures() -> TestResult {
    run_test(r#"do { let b = 3; def c [] { $b }; c }"#, "3")
}

#[test]
fn properly_nest_captures_call_first() -> TestResult {
    run_test(r#"do { let b = 3; c; def c [] { $b }; c }"#, "3")
}

#[test]
fn properly_typecheck_rest_param() -> TestResult {
    run_test(
        r#"def foo [...rest: string] { $rest | length }; foo "a" "b" "c""#,
        "3",
    )
}

#[test]
fn implied_collect_has_compatible_type() -> TestResult {
    run_test(r#"let idx = 3 | $in; $idx < 1"#, "false")
}

#[test]
fn record_expected_colon() -> TestResult {
    fail_test(r#"{ a: 2 b }"#, "expected ':'")?;
    fail_test(r#"{ a: 2 b 3 }"#, "expected ':'")
}

#[test]
fn record_missing_value() -> TestResult {
    fail_test(r#"{ a: 2 b: }"#, "expected value for record field")
}

#[test]
fn def_requires_body_closure() -> TestResult {
    fail_test("def a [] (echo 4)", "expected definition body closure")
}

#[test]
fn not_panic_with_recursive_call() {
    let result = nu!(nu_repl_code(&[
        "def px [] { if true { 3 } else { px } }",
        "let x = 1",
        "$x | px",
    ]));
    assert_eq!(result.out, "3");

    let result = nu!(nu_repl_code(&[
        "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
        "let x = 1",
        "$x | px"
    ]));
    assert_eq!(result.out, "false");

    let result = nu!(nu_repl_code(&[
        "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
        "let x = 1",
        "def foo [] { $x }",
        "foo | px"
    ]));
    assert_eq!(result.out, "false");

    let result = nu!(nu_repl_code(&[
        "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
        "let x = 1",
        "do {|| $x } | px"
    ]));
    assert_eq!(result.out, "false");

    let result = nu!(
        cwd: "tests/parsing/samples",
        "nu recursive_func_with_alias.nu"
    );
    assert!(result.status.success());
}

// https://github.com/nushell/nushell/issues/16040
#[test]
fn external_argument_with_subexpressions() -> TestResult {
    run_test(r#"^echo foo( ('bar') | $in ++ 'baz' )"#, "foobarbaz")?;
    run_test(r#"^echo foo( 'bar' )('baz')"#, "foobarbaz")?;
    run_test(r#"^echo ")('foo')(""#, ")('foo')(")?;
    fail_test(r#"^echo foo( 'bar'"#, "Unexpected end of code")
}
