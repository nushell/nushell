use crate::repl::tests::{TestResult, fail_test, run_test, run_test_contains};
use rstest::rstest;

#[test]
fn chained_operator_typecheck() -> TestResult {
    run_test("1 != 2 and 3 != 4 and 5 != 6", "true")
}

#[test]
fn type_in_list_of_this_type() -> TestResult {
    run_test(r#"42 in [41 42 43]"#, "true")
}

#[test]
fn type_in_list_of_non_this_type() -> TestResult {
    fail_test(
        r#"'hello' in [41 42 43]"#,
        "nu::parser::operator_incompatible_types",
    )
}

#[test]
fn number_int() -> TestResult {
    run_test(r#"def foo [x:number] { $x }; foo 1"#, "1")
}

#[test]
fn int_record_mismatch() -> TestResult {
    fail_test(r#"def foo [x:int] { $x }; foo {}"#, "expected int")
}

#[test]
fn number_float() -> TestResult {
    run_test(r#"def foo [x:number] { $x }; foo 1.4"#, "1.4")
}

#[test]
fn date_minus_duration() -> TestResult {
    let input = "2023-04-22 - 2day | format date %Y-%m-%d";
    let expected = "2023-04-20";
    run_test(input, expected)
}

#[test]
fn duration_minus_date_not_supported() -> TestResult {
    fail_test(
        "2day - 2023-04-22",
        "nu::parser::operator_incompatible_types",
    )
}

#[test]
fn date_plus_duration() -> TestResult {
    let input = "2023-04-18 + 2day | format date %Y-%m-%d";
    let expected = "2023-04-20";
    run_test(input, expected)
}

#[test]
fn duration_plus_date() -> TestResult {
    let input = "2024-11-10T00:00:00-00:00 + 4hr | format date";
    let expected = "Sun, 10 Nov 2024 04:00:00 +0000";
    run_test(input, expected)
}

#[test]
fn block_not_first_class_def() -> TestResult {
    fail_test(
        "def foo [x: block] { do $x }",
        "Blocks are not support as first-class values",
    )
}

#[test]
fn block_not_first_class_let() -> TestResult {
    fail_test(
        "let x: block = { 3 }",
        "Blocks are not support as first-class values",
    )
}

#[test]
fn record_subtyping() -> TestResult {
    run_test(
        "def test [rec: record<name: string, age: int>] { $rec | describe };
        test { age: 4, name: 'John' }",
        "record<age: int, name: string>",
    )
}

#[test]
fn record_subtyping_2() -> TestResult {
    run_test(
        "def test [rec: record<name: string, age: int>] { $rec | describe };
        test { age: 4, name: 'John', height: '5-9' }",
        "record<age: int, name: string, height: string>",
    )
}

#[test]
fn record_subtyping_3() -> TestResult {
    fail_test(
        "def test [rec: record<name: string, age: int>] { $rec | describe };
        test { name: 'Nu' }",
        "expected",
    )
}

#[test]
fn record_subtyping_allows_general_record() -> TestResult {
    run_test(
        "def test []: record<name: string, age: int> -> string { $in; echo 'success' };
        def underspecified []: nothing -> record {{name:'Douglas', age:42}};
        underspecified | test",
        "success",
    )
}

#[test]
fn record_subtyping_allows_record_after_general_command() -> TestResult {
    run_test(
        "def test []: record<name: string, age: int> -> string { $in; echo 'success' };
        {name:'Douglas', surname:'Adams', age:42} | select name age | test",
        "success",
    )
}

#[test]
fn record_subtyping_allows_general_inner() -> TestResult {
    run_test(
        "def merge_records [other: record<bar: int>]: record<foo: string> -> record<foo: string, bar: int> { merge $other }",
        "",
    )
}

#[test]
fn record_subtyping_works() -> TestResult {
    run_test(
        r#"def merge_records [other: record<bar: int>] { "" }; merge_records {"bar": 3, "foo": 4}"#,
        "",
    )
}

#[test]
fn transpose_into_load_env() -> TestResult {
    run_test(
        "[[col1, col2]; [a, 10], [b, 20]] | transpose --ignore-titles -r -d | load-env; $env.a",
        "10",
    )
}

#[test]
fn in_variable_expression_correct_output_type() -> TestResult {
    run_test(r#"def foo []: nothing -> string { 'foo' | $"($in)" }"#, "")
}

#[test]
fn in_variable_expression_wrong_output_type() -> TestResult {
    fail_test(
        r#"def foo []: nothing -> int { 'foo' | $"($in)" }"#,
        "expected int",
    )
}

#[rstest]
#[case("if true {} else { foo 1 }")]
#[case("if true {} else if (foo 1) == null { }")]
#[case("match 1 { 0 => { foo 1 } }")]
#[case("try { } catch { foo 1 }")]
/// type errors should propagate from `OneOf(Block | Closure | Expression, ..)`
fn in_oneof_block_expected_type(#[case] input: &str) -> TestResult {
    let def = "def foo [bar: bool] {};";

    fail_test(&format!("{def} {input}"), "expected bool")
}

#[test]
fn in_oneof_block_expected_block() -> TestResult {
    fail_test("match 1 { 0 => { try 3 } }", "expected block")
}

#[test]
fn pipeline_multiple_types() -> TestResult {
    // https://github.com/nushell/nushell/issues/15485
    run_test_contains("{year: 2019} | into datetime | date humanize", "years ago")
}

const MULTIPLE_TYPES_DEFS: &str = "
def foo []: [int -> int, int -> string] {
  if $in > 2 { 'hi' } else 4
}
def bar []: [int -> filesize, string -> string] {
  if $in == 'hi' { 'meow' } else { into filesize }
}
";

#[test]
fn pipeline_multiple_types_custom() -> TestResult {
    run_test(
        &format!(
            "{MULTIPLE_TYPES_DEFS}
            5 | foo | str trim"
        ),
        "hi",
    )
}

#[test]
fn pipeline_multiple_types_propagate_string() -> TestResult {
    run_test(
        &format!(
            "{MULTIPLE_TYPES_DEFS}
            5 | foo | bar | str trim"
        ),
        "meow",
    )
}

#[test]
fn pipeline_multiple_types_propagate_int() -> TestResult {
    run_test(
        &format!(
            "{MULTIPLE_TYPES_DEFS}
            2 | foo | bar | format filesize B"
        ),
        "4 B",
    )
}

#[test]
fn pipeline_multiple_types_propagate_error() -> TestResult {
    fail_test(
        &format!(
            "{MULTIPLE_TYPES_DEFS}
            2 | foo | bar | values"
        ),
        "parser::input_type_mismatch",
    )
}
