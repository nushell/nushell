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
    r#"expected type "directory", "path", or "glob""#
)]
// `--input` returns the site, not completions; reject flags that shape output.
#[case::input_with_detailed(
    "'ls ' | commandline complete --input --detailed",
    "cannot be used with --input"
)]
#[case::input_with_type(
    "'ls ' | commandline complete --input --type path",
    "cannot be used with --input"
)]
fn commandline_test_complete_invalid_input(
    #[case] cmd: &str,
    #[case] expected_err: &str,
) -> TestResult {
    fail_test(cmd, expected_err)
}

/// `--input` returns exactly the record a completer would be handed.
#[rstest]
#[case::command("test-", "{kind: command}")]
#[case::flag_name("test-cmd --", "{kind: flag-name}")]
#[case::flag_value("test-cmd --string ", "{kind: flag-value, flag: string}")]
#[case::short_flag_value("test-cmd -s ", "{kind: flag-value, flag: string}")]
#[case::positional("test-cmd ", "{kind: positional, index: 0}")]
#[case::variable("$ni", "{kind: variable}")]
fn commandline_test_complete_input_site(#[case] cmd: &str, #[case] expected: &str) -> TestResult {
    run_test(
        &format!(
            "
            def test-cmd [
                first?: string,
                --string(-s): string@[a],
            ] {{}}\n\
            \n\
            '{cmd}' | commandline complete --input | get site | to nuon"
        ),
        expected,
    )
}

/// `tokens` heads the record: head first, the completed token last.
#[test]
fn commandline_test_complete_input_record() -> TestResult {
    run_test(
        "def test-cmd [first?: string] {}\n\
        'test-cmd ab' | commandline complete --input | reject site | to nuon",
        "{tokens: [[text, kind, span]; \
[test-cmd, head, {start: 0, end: 8}], [ab, value, {start: 9, end: 11}]], \
context_start: 0, cursor: 11}",
    )
}

/// Resolve a completer named by an alias, through the alias.
#[rstest]
#[case::direct("comp-alias")]
#[case::nested("comp-nested")]
fn commandline_test_complete_alias_as_completer(#[case] completer: &str) -> TestResult {
    run_test(
        &format!(
            "def \"nu-complete a\" [input] {{ [alpha beta] }}\n\
            alias comp-alias = nu-complete a\n\
            alias comp-nested = comp-alias\n\
            def foo [x: string@\"{completer}\"] {{}}\n\
            'foo a' | commandline complete | to nuon"
        ),
        "[alpha]",
    )
}

/// `tokens` is never empty, even where the parser produces none.
#[rstest]
#[case::variable("$")]
#[case::open_paren("(")]
#[case::open_bracket("[")]
#[case::bare_flag("cmd --")]
#[case::cell_path("$env.")]
#[case::assignment("a=b")]
#[case::unclosed_quote("cmd \"un")]
fn commandline_test_complete_input_tokens_never_empty(#[case] line: &str) -> TestResult {
    run_test(
        &format!("('{line}' | commandline complete --input | get tokens | is-empty)"),
        "false",
    )
}

/// A failing parameter completer is empty, not the working directory; a failing external
/// one still falls back to files.
#[test]
fn commandline_test_complete_failing_completer_is_empty() -> TestResult {
    run_test(
        "def \"nu-complete boom\" [input] { error make {msg: boom} }\n\
        def foo [x: string@\"nu-complete boom\"] {}\n\
        'foo a' | commandline complete | length",
        "0",
    )
}

/// A token the parser consumes outright (a bare `--`) still appears in `tokens | last`.
#[rstest]
#[case::bare_flag("cmd --", "[[text, kind]; [cmd, head], [--, flag]]")]
#[case::trailing_slot("cmd ", r#"[[text, kind]; [cmd, head], ["", value]]"#)]
#[case::partial_flag("cmd --al", "[[text, kind]; [cmd, head], [--al, flag]]")]
fn commandline_test_complete_input_last_token(
    #[case] line: &str,
    #[case] expected: &str,
) -> TestResult {
    run_test(
        &format!(
            "def cmd [--alpha, --beta] {{}}\n\
            '{line}' | commandline complete --input | get tokens | select text kind | to nuon"
        ),
        expected,
    )
}

/// Alias-expanded tokens are not on the line; a span would point at the definition.
#[test]
fn commandline_test_complete_input_alias_tokens_have_no_span() -> TestResult {
    run_test(
        "'alias ea = ext aa; ea b' | commandline complete --input\n\
        | get tokens | each {|t| $t.span == null } | to nuon",
        "[true, true, false]",
    )
}

/// The last token is what a suggestion replaces, so a completer never needs to be told.
#[test]
fn commandline_test_complete_input_replacing() -> TestResult {
    run_test(
        "def test-cmd [first?: string] {}\n\
        'test-cmd ab' | commandline complete --input | get tokens | last | get span | to nuon",
        "{start: 9, end: 11}",
    )
}
