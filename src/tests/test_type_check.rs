use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn chained_operator_typecheck() -> TestResult {
    run_test("1 != 2 && 3 != 4 && 5 != 6", "true")
}

#[test]
fn type_in_list_of_this_type() -> TestResult {
    run_test(r#"42 in [41 42 43]"#, "true")
}

#[test]
fn type_in_list_of_non_this_type() -> TestResult {
    fail_test(r#"'hello' in [41 42 43]"#, "mismatched for operation")
}
