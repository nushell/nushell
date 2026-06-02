use rstest::rstest;

use crate::repl::tests::{TestResult, fail_test, run_test};

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
fn commandline_test_accepted_command() -> TestResult {
    run_test(
        "commandline edit --accept \"print accepted\"\n | commandline",
        "print accepted",
    )
}

#[test]
fn commandline_test_complete_input() -> TestResult {
    run_test(
        "def test-bar [] {}\n\
        def test-baz [] {}\n\
        'test-' | commandline complete | to nuon",
        "[test-bar, test-baz]",
    )
}

#[test]
fn commandline_test_complete_no_input() -> TestResult {
    run_test(
        "def test-bar [] {}\n\
        def test-baz [] {}\n\
        commandline edit --replace 'test-'\n\
        commandline complete | to nuon",
        "[test-bar, test-baz]",
    )
}

#[test]
fn commandline_test_complete_flags() -> TestResult {
    run_test(
        "def cmd [ --flag: string, --switch(-s) ] {}\n\
        'cmd -' | commandline complete | to nuon",
        "[--flag, --help, --switch, -h, -s]",
    )
}

#[test]
fn commandline_test_complete_reentrant() -> TestResult {
    run_test(
        "def recurse [a: string@[a, b, c]] {\n\
            'recurse ' | commandline complete\n\
        }\n\
        def wrapped [arg:string@recurse] {}\n\
        \n\
        'wrapped ' | commandline complete | to nuon",
        "[a, b, c]",
    )
}

#[rstest]
#[case::cmd(
    "test-",
    r#"{value: test-cmd, span: {start: 0, end: 5}, description: "", kind: command, type: custom}"#
)]
#[case::int(
    "test-cmd --int ",
    r#"{value: "1", span: {start: 15, end: 15}, kind: value, type: int}"#
)]
#[case::string(
    "test-cmd --string ",
    "{value: a, span: {start: 18, end: 18}, kind: value, type: string}"
)]
fn commandline_test_complete_detailed(#[case] cmd: &str, #[case] expected: &str) -> TestResult {
    run_test(
        &format!(
            "
            def complete-int [] {{ [ 1 ] }}
            def test-cmd [
                --int: int@complete-int,
                --string: string@[a],
            ] {{}}\n\
            \n\
            '{cmd}' | commandline complete --detailed | first | to nuon"
        ),
        expected,
    )
}

#[rstest]
#[case::invalid_input("123 | commandline complete", "command doesn't support int input")]
#[case::invalid_type(
    "commandline complete --type foo",
    r#"expected type "directory", "path", or "glob", but got "foo""#
)]
fn commandline_test_complete_invalid_input(
    #[case] cmd: &str,
    #[case] expected_err: &str,
) -> TestResult {
    fail_test(cmd, expected_err)
}
