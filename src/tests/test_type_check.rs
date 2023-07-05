use crate::tests::{fail_test, run_test, TestResult};

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
    let input = "2023-04-22 - 2day | date format %Y-%m-%d";
    let expected = "2023-04-20";
    run_test(input, expected)
}

#[test]
fn date_plus_duration() -> TestResult {
    let input = "2023-04-18 + 2day | date format %Y-%m-%d";
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
        "record<name: string, age: int>",
    )
}
