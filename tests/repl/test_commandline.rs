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

/// The resolved cursor at each kind of site, as `--input` reports it.
#[rstest]
#[case::command("test-", "{kind: command}")]
#[case::flag_name("test-cmd --", "{kind: flag-name}")]
#[case::flag_value("test-cmd --string ", "{kind: flag-value, flag: string}")]
#[case::short_flag_value("test-cmd -s ", "{kind: flag-value, flag: string}")]
#[case::positional("test-cmd ", "{kind: positional, index: 0}")]
#[case::variable("$ni", "{kind: variable}")]
fn commandline_test_complete_input_cursor(#[case] cmd: &str, #[case] expected: &str) -> TestResult {
    run_test(
        &format!(
            "
            def test-cmd [
                first?: string,
                --string(-s): string@[a],
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
place: {cursor: 11, target: {start: 9, end: 11}, kind: positional, index: 0}}",
    )
}

/// A cursor inside a closure hangs its command off the nesting token, tagged with the slot
/// it fills. `ignored` is beside the cursor, not around it, so it is nowhere in the tree.
#[test]
fn commandline_test_complete_input_contexts() -> TestResult {
    run_test(
        "def test-cmd [first?: string] {}\n\
        'ignored | each { test-cmd ab' | commandline complete --input-full | to nuon",
        "{contexts: {tokens: [[text, kind, span, nested]; \
[each, head, {start: 10, end: 14}, null], \
[null, block, {start: 15, end: 28}, \
{kind: positional, index: 0, tokens: [[text, kind, span, nested]; \
[test-cmd, head, {start: 17, end: 25}, null], [ab, value, {start: 26, end: 28}, null]]}]]}, \
place: {cursor: {path: [1, 1], byte: 2}, \
target: {start: {path: [1, 1], byte: 0}, end: {path: [1, 1], byte: 2}}, \
kind: positional, index: 0}}",
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

/// Alias-expanded tokens are not on the line; a span would point at the definition.
#[test]
fn commandline_test_complete_input_alias_tokens_have_no_span() -> TestResult {
    run_test(
        "'alias ea = ext aa; ea b' | commandline complete --input-full\n\
        | get contexts.tokens | each {|t| $t.span == null } | to nuon",
        "[true, true, false]",
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
kind: positional, index: 0}}}}",
            cursor = line.len()
        ),
    )
}

/// The gap inside a closure belongs to the command the gap is in, not the one enclosing it:
/// `each { ls ⌶` completes `ls`'s argument, and a suggestion replaces the empty slot rather
/// than the whole closure.
#[test]
fn commandline_test_complete_input_gap_resolves_inside_the_closure() -> TestResult {
    run_test(
        "'each { ls ' | commandline complete --input-full\n\
        | {head: $in.contexts.tokens.1.nested.tokens.0.text, kind: $in.place.kind} | to nuon",
        "{head: ls, kind: positional}",
    )
}

/// Nesting is as deep as the cursor goes. A row condition holds a subexpression holding an
/// external call, and each level is tagged with the slot the next one fills — so an external
/// completer three levels down still resolves to the argument it is really completing.
#[test]
fn commandline_test_complete_input_nests_as_deep_as_the_cursor() -> TestResult {
    run_test(
        "'ls | where a == (^ext ' | commandline complete --input-full | {\n\
            heads: [$in.contexts.tokens.0.text\n\
                    $in.contexts.tokens.1.nested.tokens.0.text\n\
                    $in.contexts.tokens.1.nested.tokens.2.nested.tokens.0.text]\n\
            slots: [$in.contexts.tokens.1.nested.kind\n\
                    $in.contexts.tokens.1.nested.tokens.2.nested.kind]\n\
            place: $in.place\n\
        } | to nuon",
        "{heads: [where, a, ext], slots: [positional, operator], \
place: {cursor: {path: [1, 2, 1], byte: 0}, \
target: {start: {path: [1, 2, 1], byte: 0}, end: {path: [1, 2, 1], byte: 0}}, \
kind: external-arg, index: 0}}",
    )
}

/// Every offset is a byte offset, in walks as in spans: `é` is two bytes, so the cursor
/// after `héllo` is six into the token and not five.
#[test]
fn commandline_test_complete_input_offsets_are_bytes() -> TestResult {
    run_test(
        "'str replace héllo' | commandline complete --input-full\n\
        | {token: $in.contexts.tokens.1.text, byte: $in.place.cursor.byte} | to nuon",
        "{token: héllo, byte: 6}",
    )
}

/// An empty line still resolves: a completer reads the head it is about to complete rather
/// than a null it has to guard against.
#[rstest]
#[case::token(
    "--input",
    "{token: {text: \"\", kind: head, span: {start: 0, end: 0}}, \
place: {cursor: 0, target: {start: 0, end: 0}, kind: command}}"
)]
#[case::full(
    "--input-full",
    "{contexts: {tokens: [[text, kind, span, nested]; [\"\", head, {start: 0, end: 0}, null]]}, \
place: {cursor: {path: [0], byte: 0}, \
target: {start: {path: [0], byte: 0}, end: {path: [0], byte: 0}}, kind: command}}"
)]
fn commandline_test_complete_input_empty_line(
    #[case] flag: &str,
    #[case] expected: &str,
) -> TestResult {
    run_test(
        &format!("'' | commandline complete {flag} | to nuon"),
        expected,
    )
}

/// A target may run across tokens: completing a cell path replaces the whole path, so its
/// walks land on different tokens while the cursor stays on the last.
#[test]
fn commandline_test_complete_input_target_spans_tokens() -> TestResult {
    run_test(
        "'ls | get na.fo' | commandline complete --input-full\n\
        | {tokens: $in.contexts.tokens.text, target: $in.place.target} | to nuon",
        "{tokens: [get, na, fo], \
target: {start: {path: [1], byte: 0}, end: {path: [2], byte: 2}}}",
    )
}
