use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn commandline_test_get_empty() -> TestResult {
    run_test("commandline", "")
}

#[test]
fn commandline_test_append() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '2'\ncommandline --append \
         'ab'\nprint (commandline)\ncommandline --cursor",
        "0👩‍❤️‍👩2ab\n2",
    )
}

#[test]
fn commandline_test_insert() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '2'\ncommandline --insert \
         'ab'\nprint (commandline)\ncommandline --cursor",
        "0👩‍❤️‍👩ab2\n4",
    )
}

#[test]
fn commandline_test_replace() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --replace 'ab'\nprint \
         (commandline)\ncommandline --cursor",
        "ab\n2",
    )
}

#[test]
fn commandline_test_cursor() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '1'\ncommandline --insert \
         'x'\ncommandline",
        "0x👩‍❤️‍👩2",
    )?;
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '2'\ncommandline --insert \
         'x'\ncommandline",
        "0👩‍❤️‍👩x2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_begin() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩'\ncommandline --cursor '0'\ncommandline --cursor",
        "0",
    )
}

#[test]
fn commandline_test_cursor_show_pos_end() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩'\ncommandline --cursor '2'\ncommandline --cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_mid() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '1'\ncommandline --cursor",
        "1",
    )?;
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\ncommandline --cursor '2'\ncommandline --cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_too_small() -> TestResult {
    run_test(
        "commandline --replace '123456'\ncommandline --cursor '-1'\ncommandline --insert \
         '0'\ncommandline",
        "0123456",
    )
}

#[test]
fn commandline_test_cursor_too_large() -> TestResult {
    run_test(
        "commandline --replace '123456'\ncommandline --cursor '10'\ncommandline --insert \
         '0'\ncommandline",
        "1234560",
    )
}

#[test]
fn commandline_test_cursor_invalid() -> TestResult {
    fail_test(
        "commandline --replace '123456'\ncommandline --cursor 'abc'",
        r#"string "abc" does not represent a valid int"#,
    )
}
