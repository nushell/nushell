use crate::tests::{run_test, TestResult};

#[test]
fn commandline_test_get_empty() -> TestResult {
    run_test("commandline", "")
}

#[test]
fn commandline_test_append() -> TestResult {
    run_test(
        "commandline --append '123'\n\
        commandline\n\
        commandline --append '456'\n\
        commandline",
        "123\n\
        123456",
    )
}

#[test]
fn commandline_test_insert() -> TestResult {
    run_test(
        "commandline --insert '123'\n\
        commandline\n\
        commandline --insert '456'\n\
        commandline",
        "123\n\
        456123",
    )
}

#[test]
fn commandline_test_replace() -> TestResult {
    run_test(
        "commandline --append '123'\n\
        commandline\n\
        commandline --replace '456'\n\
        commandline",
        "123\n\
        456",
    )
}
