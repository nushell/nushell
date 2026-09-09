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
// Detailed output includes suggestion metadata.
#[case::cmd(
    "test-",
    r#"{value: test-cmd, span: {start: 0, end: 5}, description: "", append_whitespace: true, match_indices: [0, 1, 2, 3, 4], kind: command, type: custom}"#
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
    "one of directory, path, glob, command, variable, env-var"
)]
// `--input` returns the completer input record, not completions; reject flags that shape output.
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

/// `--input` reports the resolved cursor and declared shape.
#[rstest]
#[case::command("test-", "{kind: command}")]
#[case::flag_name("test-cmd --", "{kind: flag-name}")]
#[case::flag_value(
    "test-cmd --string ",
    "{kind: flag-value, flag: string, shape: string}"
)]
#[case::short_flag_value("test-cmd -s ", "{kind: flag-value, flag: string, shape: string}")]
// Short-only flags still report their declared shape.
#[case::short_only_flag_value("test-cmd -e ", "{kind: flag-value, flag: e, shape: directory}")]
#[case::positional("test-cmd ", "{kind: positional, index: 0, shape: string}")]
#[case::variable("$ni", "{kind: variable}")]
fn commandline_test_complete_input_cursor(#[case] cmd: &str, #[case] expected: &str) -> TestResult {
    run_test(
        &format!(
            "
            def test-cmd [
                first?: string,
                --string(-s): string@[a],
                -e: directory,
            ] {{}}\n\
            \n\
            '{cmd}' | commandline complete --input \
            | get place | reject cursor target | to nuon"
        ),
        expected,
    )
}

/// The whole record for a flat commandline: one context, head first and the completed
/// token last, and both walks landing on that token.
#[test]
fn commandline_test_complete_input_record() -> TestResult {
    run_test(
        "def test-cmd [first?: string] {}\n\
        'test-cmd ab' | commandline complete --input | to nuon",
        "{token: {text: ab, kind: value, span: {start: 9, end: 11}}, \
place: {cursor: 11, target: {start: 9, end: 11}, kind: positional, index: 0, shape: string}, \
buffer: \"test-cmd ab\"}",
    )
}

/// `buffer` is the whole line up to the cursor, with `place` resolving against the command
/// the cursor is actually in — here `test-cmd`'s positional, even though the cursor is nested
/// in a closure.
#[test]
fn commandline_test_complete_input_buffer() -> TestResult {
    run_test(
        "def test-cmd [first?: string] {}\n\
        'ignored | each { test-cmd ab' | commandline complete --input | reject token | to nuon",
        "{place: {cursor: 28, target: {start: 26, end: 28}, kind: positional, index: 0, shape: string}, \
buffer: \"ignored | each { test-cmd ab\"}",
    )
}

/// `buffer` is the line being completed, so it carries the whole line up to the cursor even
/// where the `commandline` command comes back empty -- e.g. the fresh command after a `;`.
/// This is why an external completer reads `buffer` instead of calling `commandline`.
#[test]
fn commandline_test_complete_input_buffer_survives_a_semicolon() -> TestResult {
    run_test(
        "'foo; ' | commandline complete --input\n\
        | {buffer: $in.buffer, kind: $in.place.kind} | to nuon",
        "{buffer: \"foo; \", kind: command}",
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
        &format!("('{line}' | commandline complete --input | get token | is-empty)"),
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

/// A token the parser consumes outright (a bare `--`) still gets a slot, so `cursor.token`
/// and `replacing` always have one to point at.
#[rstest]
#[case::bare_flag("cmd --", "{text: --, kind: flag}")]
#[case::trailing_slot("cmd ", r#"{text: "", kind: value}"#)]
#[case::partial_flag("cmd --al", "{text: --al, kind: flag}")]
fn commandline_test_complete_input_last_token(
    #[case] line: &str,
    #[case] expected: &str,
) -> TestResult {
    run_test(
        &format!(
            "def cmd [--alpha, --beta] {{}}\n\
            '{line}' | commandline complete --input \
            | get token | select text kind | to nuon"
        ),
        expected,
    )
}

/// A closure flattens to its delimiters and the padding around them, so the cursor in a
/// trailing gap must not land on whitespace: `each { ls ⌶` is completing `ls`'s first
/// argument, an empty slot, and never the space before it.
#[rstest]
#[case::closure("each { ls ")]
#[case::nested_closure("do { each { ls ")]
#[case::past_a_pipe("ls | each { ls ")]
fn commandline_test_complete_input_skips_whitespace_tokens(#[case] line: &str) -> TestResult {
    run_test(
        &format!("'{line}' | commandline complete --input | to nuon"),
        &format!(
            "{{token: {{text: \"\", kind: value, span: {{start: {cursor}, end: {cursor}}}}}, \
place: {{cursor: {cursor}, target: {{start: {cursor}, end: {cursor}}}, \
kind: positional, index: 0, shape: \"oneof<glob, string>\"}}, buffer: \"{line}\"}}",
            cursor = line.len()
        ),
    )
}

/// The gap inside a closure belongs to the command the gap is in, not the one enclosing it:
/// the buffer spans the whole closure, but `place` resolves to `ls`'s argument (its shape),
/// never `each`'s block.
#[test]
fn commandline_test_complete_input_gap_resolves_inside_the_closure() -> TestResult {
    run_test(
        "'each { ls ' | commandline complete --input\n\
        | {buffer: $in.buffer, kind: $in.place.kind, shape: $in.place.shape} | to nuon",
        "{buffer: \"each { ls \", kind: positional, shape: \"oneof<glob, string>\"}",
    )
}

/// Resolution is as deep as the cursor goes: an external call inside a subexpression inside a
/// pipeline still resolves to the argument it is really completing, while `buffer` keeps the
/// whole line up to the cursor.
#[test]
fn commandline_test_complete_input_resolves_deep_in_subexpressions() -> TestResult {
    run_test(
        "'ls | where a == (^ext ' | commandline complete --input\n\
        | {buffer: $in.buffer, place: ($in.place | reject cursor target)} | to nuon",
        "{buffer: \"ls | where a == (^ext \", place: {kind: external-arg, index: 0}}",
    )
}

/// Every offset is a byte offset: `é` is two bytes, so the `héllo` token — and the target
/// that replaces it — is six bytes wide, not five.
#[test]
fn commandline_test_complete_input_offsets_are_bytes() -> TestResult {
    run_test(
        "'str replace héllo' | commandline complete --input\n\
        | {token: $in.token.text, width: ($in.place.target.end - $in.place.target.start)} | to nuon",
        "{token: héllo, width: 6}",
    )
}

/// An empty line still resolves: a completer reads the head it is about to complete rather
/// than a null it has to guard against, and the buffer is just empty.
#[test]
fn commandline_test_complete_input_empty_line() -> TestResult {
    run_test(
        "'' | commandline complete --input | to nuon",
        "{token: {text: \"\", kind: head, span: {start: 0, end: 0}}, \
place: {cursor: 0, target: {start: 0, end: 0}, kind: command}, buffer: \"\"}",
    )
}

/// A target may run across tokens, and the `token` runs with it: completing a cell path
/// replaces the whole path, so the whole path is what is being completed. A completer that
/// echoes `$token.text` back leaves the line as it found it, rather than dropping the
/// `na.` its answer would have overwritten.
#[test]
fn commandline_test_complete_input_target_spans_tokens() -> TestResult {
    run_test(
        "'ls | get na.fo' | commandline complete --input\n\
        | {token: $in.token.text, target: $in.place.target} | to nuon",
        "{token: \"na.fo\", target: {start: 9, end: 14}}",
    )
}

/// `--type` selects a built-in completion source.
#[rstest]
#[case::command(
    "def my-unique-cmd [] {}",
    "'my-unique' | commandline complete --type command",
    "[my-unique-cmd]"
)]
#[case::variable(
    "",
    "'$n' | commandline complete --type variable | where $it == '$nu'",
    r#"["$nu"]"#
)]
#[case::env_var(
    "$env.MY_UNIQUE_EV = 1",
    "'MY_UNIQUE' | commandline complete --type env-var",
    "[MY_UNIQUE_EV]"
)]
fn commandline_test_complete_type_sources(
    #[case] setup: &str,
    #[case] complete: &str,
    #[case] expected: &str,
) -> TestResult {
    run_test(&format!("{setup}\n{complete} | to nuon"), expected)
}

/// `fallback: true` merges custom and command-wide completions.
#[test]
fn commandline_test_complete_fallthrough_stacks_completers() -> TestResult {
    run_test(
        "def per-arg [token] { {completions: [static-a], fallback: true} }\n\
        def cmd-wide [token] { [wide-x] }\n\
        @complete cmd-wide\n\
        def --wrapped my-cmd [x: string@per-arg, ...rest] {}\n\
        'my-cmd ' | commandline complete | to nuon",
        "[static-a, wide-x]",
    )
}

/// Without `fallback`, the parameter completer claims the slot.
#[test]
fn commandline_test_complete_without_fallthrough_claims_the_slot() -> TestResult {
    run_test(
        "def per-arg [token] { [static-a] }\n\
        def cmd-wide [token] { [wide-x] }\n\
        @complete cmd-wide\n\
        def --wrapped my-cmd [x: string@per-arg, ...rest] {}\n\
        'my-cmd ' | commandline complete | to nuon",
        "[static-a]",
    )
}

/// Custom suggestions preserve detailed metadata.
#[rstest]
#[case::kind("kind: directory", "get kind", "directory")]
#[case::append_whitespace("append_whitespace: true", "get append_whitespace", "true")]
fn commandline_test_complete_custom_suggestion_fields(
    #[case] field: &str,
    #[case] project: &str,
    #[case] expected: &str,
) -> TestResult {
    run_test(
        &format!(
            "def comp [token] {{ [{{value: foo, {field}}}] }}\n\
            def my-cmd [x: string@comp] {{}}\n\
            'my-cmd f' | commandline complete --detailed | first | {project}"
        ),
        expected,
    )
}
