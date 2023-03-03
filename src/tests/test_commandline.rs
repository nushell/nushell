use crate::tests::{fail_test, run_test, TestResult};

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

#[test]
fn commandline_test_cursor() -> TestResult {
    run_test(
        "commandline --replace '0😀2'\n\
        commandline --cursor '1' \n\
        commandline --insert 'x'\n\
        commandline",
        "0x😀2",
    )?;
    run_test(
        "commandline --replace '0😀2'\n\
        commandline --cursor '2' \n\
        commandline --insert 'x'\n\
        commandline",
        "0😀x2",
    )
}

#[test]
fn commandline_test_cursor_show_pos() -> TestResult {
    run_test(
        "commandline --replace '0😀2'\n\
        commandline --cursor '1' \n\
        commandline --cursor",
        "1",
    )?;
    run_test(
        "commandline --replace '0😀2'\n\
        commandline --cursor '2' \n\
        commandline --cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_too_small() -> TestResult {
    run_test(
        "commandline --replace '123456'\n\
        commandline --cursor '-1' \n\
        commandline --insert '0'\n\
        commandline",
        "0123456",
    )
}

#[test]
fn commandline_test_cursor_too_large() -> TestResult {
    run_test(
        "commandline --replace '123456'\n\
        commandline --cursor '10' \n\
        commandline --insert '0'\n\
        commandline",
        "1234560",
    )
}

#[test]
fn commandline_test_cursor_invalid() -> TestResult {
    fail_test(
        "commandline --replace '123456'\n\
        commandline --cursor 'abc'",
        r#"string "abc" does not represent a valid integer"#,
    )
}
