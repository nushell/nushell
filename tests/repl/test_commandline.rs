use crate::repl::tests::{TestResult, fail_test, run_test};

#[test]
fn commandline_test_get_empty() -> TestResult {
    run_test("commandline", "")
}

#[test]
fn commandline_test_append() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 2\ncommandline edit --append \
         'ab'\nprint (commandline)\ncommandline get-cursor",
        "0👩‍❤️‍👩2ab\n2",
    )
}

#[test]
fn commandline_test_insert() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 2\ncommandline edit --insert \
         'ab'\nprint (commandline)\ncommandline get-cursor",
        "0👩‍❤️‍👩ab2\n4",
    )
}

#[test]
fn commandline_test_replace() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline edit --replace 'ab'\nprint \
         (commandline)\ncommandline get-cursor",
        "ab\n2",
    )
}

#[test]
fn commandline_test_cursor() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 1\ncommandline edit --insert \
         'x'\ncommandline",
        "0x👩‍❤️‍👩2",
    )?;
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 2\ncommandline edit --insert \
         'x'\ncommandline",
        "0👩‍❤️‍👩x2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_begin() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩'\ncommandline set-cursor 0\ncommandline get-cursor",
        "0",
    )
}

#[test]
fn commandline_test_cursor_show_pos_end() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩'\ncommandline set-cursor 2\ncommandline get-cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_mid() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 1\ncommandline get-cursor",
        "1",
    )?;
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\ncommandline set-cursor 2\ncommandline get-cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_too_small() -> TestResult {
    run_test(
        "commandline edit --replace '123456'\ncommandline set-cursor -1\ncommandline edit \
         --insert '0'\ncommandline",
        "0123456",
    )
}

#[test]
fn commandline_test_cursor_too_large() -> TestResult {
    run_test(
        "commandline edit --replace '123456'\ncommandline set-cursor 10\ncommandline edit \
         --insert '0'\ncommandline",
        "1234560",
    )
}

#[test]
fn commandline_test_cursor_invalid() -> TestResult {
    fail_test(
        "commandline edit --replace '123456'\ncommandline set-cursor 'abc'",
        "expected int",
    )
}

#[test]
fn commandline_test_cursor_end() -> TestResult {
    run_test(
        "commandline edit --insert '🤔🤔'; commandline set-cursor --end; commandline get-cursor",
        "2", // 2 graphemes
    )
}

#[test]
fn commandline_test_cursor_type() -> TestResult {
    run_test("commandline get-cursor | describe", "int")
}

#[test]
fn commandline_test_accepted_command() -> TestResult {
    run_test(
        "commandline edit --accept \"print accepted\"\n | commandline",
        "print accepted",
    )
}
