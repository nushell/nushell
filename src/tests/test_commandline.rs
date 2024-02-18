use crate::tests::{fail_test, run_test, TestResult};
use nu_test_support::nu;

#[test]
fn commandline_test_get_empty() -> TestResult {
    run_test("commandline", "")
}

#[test]
fn commandline_test_append() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 2\n\
        commandline edit --append 'ab'\n\
        print (commandline)\n\
        commandline get-cursor",
        "0👩‍❤️‍👩2ab\n\
        2",
    )
}

#[test]
fn commandline_test_insert() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 2\n\
        commandline edit --insert 'ab'\n\
        print (commandline)\n\
        commandline get-cursor",
        "0👩‍❤️‍👩ab2\n\
        4",
    )
}

#[test]
fn commandline_test_replace() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline edit --replace 'ab'\n\
        print (commandline)\n\
        commandline get-cursor",
        "ab\n\
        2",
    )
}

#[test]
fn commandline_test_cursor() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 1\n\
        commandline edit --insert 'x'\n\
        commandline",
        "0x👩‍❤️‍👩2",
    )?;
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 2\n\
        commandline edit --insert 'x'\n\
        commandline",
        "0👩‍❤️‍👩x2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_begin() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩'\n\
        commandline set-cursor 0\n\
        commandline get-cursor",
        "0",
    )
}

#[test]
fn commandline_test_cursor_show_pos_end() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩'\n\
        commandline set-cursor 2\n\
        commandline get-cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_show_pos_mid() -> TestResult {
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 1\n\
        commandline get-cursor",
        "1",
    )?;
    run_test(
        "commandline edit --replace '0👩‍❤️‍👩2'\n\
        commandline set-cursor 2\n\
        commandline get-cursor",
        "2",
    )
}

#[test]
fn commandline_test_cursor_too_small() -> TestResult {
    run_test(
        "commandline edit --replace '123456'\n\
        commandline set-cursor -1\n\
        commandline edit --insert '0'\n\
        commandline",
        "0123456",
    )
}

#[test]
fn commandline_test_cursor_too_large() -> TestResult {
    run_test(
        "commandline edit --replace '123456'\n\
        commandline set-cursor 10\n\
        commandline edit --insert '0'\n\
        commandline",
        "1234560",
    )
}

#[test]
fn commandline_test_cursor_invalid() -> TestResult {
    fail_test(
        "commandline edit --replace '123456'\n\
        commandline set-cursor 'abc'",
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
fn deprecated_commandline_test_append() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '2'\n\
        commandline --append 'ab'\n\
        print (commandline)\n\
        commandline --cursor",
        "0👩‍❤️‍👩2ab\n\
        2",
    )
}

#[test]
fn deprecated_commandline_test_insert() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '2'\n\
        commandline --insert 'ab'\n\
        print (commandline)\n\
        commandline --cursor",
        "0👩‍❤️‍👩ab2\n\
        4",
    )
}

#[test]
fn deprecated_commandline_test_replace() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --replace 'ab'\n\
        print (commandline)\n\
        commandline --cursor",
        "ab\n\
        2",
    )
}

#[test]
fn deprecated_commandline_test_cursor() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '1'\n\
        commandline --insert 'x'\n\
        commandline",
        "0x👩‍❤️‍👩2",
    )?;
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '2'\n\
        commandline --insert 'x'\n\
        commandline",
        "0👩‍❤️‍👩x2",
    )
}

#[test]
fn deprecated_commandline_test_cursor_show_pos_begin() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩'\n\
        commandline --cursor '0'\n\
        commandline --cursor",
        "0",
    )
}

#[test]
fn deprecated_commandline_test_cursor_show_pos_end() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩'\n\
        commandline --cursor '2'\n\
        commandline --cursor",
        "2",
    )
}

#[test]
fn deprecated_commandline_test_cursor_show_pos_mid() -> TestResult {
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '1'\n\
        commandline --cursor",
        "1",
    )?;
    run_test(
        "commandline --replace '0👩‍❤️‍👩2'\n\
        commandline --cursor '2'\n\
        commandline --cursor",
        "2",
    )
}

#[test]
fn deprecated_commandline_test_cursor_too_small() -> TestResult {
    run_test(
        "commandline --replace '123456'\n\
        commandline --cursor '-1'\n\
        commandline --insert '0'\n\
        commandline",
        "0123456",
    )
}

#[test]
fn deprecated_commandline_test_cursor_too_large() -> TestResult {
    run_test(
        "commandline --replace '123456'\n\
        commandline --cursor '10'\n\
        commandline --insert '0'\n\
        commandline",
        "1234560",
    )
}

#[test]
fn deprecated_commandline_test_cursor_invalid() -> TestResult {
    fail_test(
        "commandline --replace '123456'\n\
        commandline --cursor 'abc'",
        r#"string "abc" does not represent a valid int"#,
    )
}

#[test]
fn deprecated_commandline_test_cursor_end() -> TestResult {
    run_test(
        "commandline --insert '🤔🤔'; commandline --cursor-end; commandline --cursor",
        "2", // 2 graphemes
    )
}

#[test]
fn deprecated_commandline_flag_cursor_get() {
    let actual = nu!("commandline --cursor");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_flag_cursor_set() {
    let actual = nu!("commandline -c 0");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_flag_cursor_end() {
    let actual = nu!("commandline --cursor-end");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_flag_append() {
    let actual = nu!("commandline --append 'abc'");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_flag_insert() {
    let actual = nu!("commandline --insert 'abc'");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_flag_replace() {
    let actual = nu!("commandline --replace 'abc'");
    assert!(actual.err.contains("deprecated"));
}

#[test]
fn deprecated_commandline_replace_current_buffer() {
    let actual = nu!("commandline 'abc'");
    assert!(actual.err.contains("deprecated"));
}
