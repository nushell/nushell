use crate::repl::tests::{fail_test, run_test, TestResult};

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
    fail_test(r#"'hello' in [41 42 43]"#, "is not supported")
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
fn date_plus_duration() -> TestResult {
    let input = "2023-04-18 + 2day | format date %Y-%m-%d";
    let expected = "2023-04-20";
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
