//! Regression tests for the `winnow-parser` front end (blocks): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::engine::StateWorkingSet;
use nu_test_support::{fs::Stub, playground::Playground, prelude::*};
use rstest::rstest;

/// `count` statements that each build a closure, about 45 bytes apiece: enough of them make a
/// block long enough to be parsed ahead (`AHEAD_MIN_BYTES` in
/// `crates/nu-parser/src/winnow/driver.rs`).
fn filler(count: usize) -> String {
    (1..=count)
        .map(|i| format!("    let x{i} = [1 2 3] | each {{|it| $it + {i} }}\n"))
        .collect()
}

/// A call through an alias of `overlay use` (or of `overlay`, with the subcommand after it) in
/// a nested block brings in commands the rest of the block calls: once the classic parser has
/// applied it, the classic parser parses the rest of the block too, as it does in a file.
#[rstest]
#[case::alias_of_overlay_use("alias ou = overlay use", "ou spam.nu")]
#[case::alias_of_overlay("alias o = overlay", "o use spam.nu")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn overlay_through_an_alias_in_a_nested_block(#[case] alias: &str, #[case] call: &str) -> Result {
    Playground::setup("winnow_overlay_through_an_alias", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "spam.nu",
            r#"export def "foo bar" [] { "from overlay" }"#,
        )]);
        let code = format!(
            r#"def foo [x?] {{ $"local foo with ($x)" }}
{alias}
do {{
    {call}
    foo bar
}}"#
        );
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("from overlay")
    })
}

/// A statement the classic parser parses alone keeps the line end after it, past the spaces,
/// tabs and `\r` the lexer skips, as in the whole block, where a `|` that ends an alias's value
/// before a line end is no pipeline missing its end.
#[rstest]
#[case::crlf("alias l = ls |\r\n\r\n'ok'")]
#[case::trailing_space("alias l = ls | \n\n'ok'")]
#[case::trailing_tab("alias l = ls |\t\n\n'ok'")]
#[case::crlf_in_a_block("do {\r\n    alias l = ls |\r\n}\r\n'ok'")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn dangling_pipe_before_a_line_end(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("ok")
}

/// The classic lite parser keeps an `e>|` with the element it ends, as it keeps a `|`; a `|`
/// after it starts an empty command, which hands that `|` on to the next element.
#[rstest]
#[case::next_line("^ls e>|\n# c\n| lines")]
#[case::same_line("^ls e>| | lines")]
#[case::after_a_file_redirection("^ls o> out.txt e>|\n| lines")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn pipe_after_an_err_pipe(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    let pipes: Vec<_> = block.pipelines[0]
        .elements
        .iter()
        .map(|element| element.pipe.map(|pipe| working_set.get_span_contents(pipe)))
        .collect();
    assert_eq!(pipes, [Some(&b"e>|"[..]), Some(&b"|"[..])], "`{code}`");
    Ok(())
}

/// In parentheses newlines separate nothing: a statement the classic parser parses alone is
/// lexed as `parse_full_cell_path` lexes the subexpression, newlines as whitespace.
#[rstest]
#[case::closure_on_the_next_line("(\n  [1 2 3 4] | take while\n    {|x| $x < 3}\n)", [1, 2])]
#[case::argument_on_the_next_line("(\n  %echo 1\n    2\n)", [1, 2])]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn statement_in_parentheses_goes_on_over_lines(
    #[case] code: &str,
    #[case] expected: [i64; 2],
) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// A subexpression with a `use` is parsed again statement by statement, its statements lexed
/// as in the subexpression: the `use` takes the names on the lines after it.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn use_in_parentheses_goes_on_over_lines() -> Result {
    Playground::setup("winnow_use_in_parentheses", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "m.nu",
            r#"export def equal [] { "imported" }"#,
        )]);
        let code = "(\n  use m.nu\n    equal\n  ; equal\n)";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("imported")
    })
}

/// The value of a binding or an assignment is a subexpression too, but one pipeline: given
/// back to the classic parser (a `%` call always is), it parses the same over lines and
/// comments as in the block.
#[rstest]
#[case::let_value("let r = [1 2 3 4] | # the first two\n  %take 2\n$r")]
#[case::assignment_value("mut r = [1 2 3 4]\n$r = $r | # the first two\n  %take 2\n$r")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn binding_value_given_back_over_lines(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq([1, 2])
}

/// `%name` calls a built-in that a custom command shadows, as in the classic `parse_call`: the
/// winnow parser takes it without a syntax error (which hands the rest of the block to the
/// classic parser) and, parsed ahead, without an answer about the built-in that changes once
/// the block's definitions are declared (which does too).
#[rstest]
#[case::parsed_here(0)]
#[case::parsed_ahead(100)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn percent_call_of_a_shadowed_builtin(#[case] statements: usize) -> Result {
    let code = format!(
        "def echo [] {{ 'custom' }}\nlet n = (%echo 5)\n{}$n",
        filler(statements)
    );
    // Made first, so that the counters take only this code's parse.
    let mut tester = test();
    let before = nu_parser::winnow_stats();
    tester.run(code).expect_value_eq(5)?;
    let after = nu_parser::winnow_stats();
    assert_eq!(after.error_statements, before.error_statements);
    assert_eq!(after.ahead_fallbacks, before.ahead_fallbacks);
    if statements > 0 && std::thread::available_parallelism().is_ok_and(|cpus| cpus.get() > 1) {
        assert!(
            after.ahead_runs > before.ahead_runs,
            "the block was not parsed ahead"
        );
    }
    Ok(())
}

/// `run` declares the script's commands where it is parsed, as the classic parser does it: a
/// nested block with a `run` is parsed again statement by statement, so the statements after
/// it see them.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn run_in_a_nested_block() -> Result {
    Playground::setup("winnow_run_in_a_nested_block", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "r.nu",
            r#"def "foo bar" [] { "from run" }"#,
        )]);
        let code = r#"def foo [x?] { $"local foo with ($x)" }
do {
    run r.nu
    foo bar
}"#;
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("from run")
    })
}

/// The classic parser takes `source` by the first word of a statement, whatever custom command
/// has a longer name: a nested block whose statement starts with `source` is parsed again
/// statement by statement, so the statements after it see what the file declares.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn source_shadowed_by_a_custom_command_in_a_nested_block() -> Result {
    Playground::setup(
        "winnow_source_shadowed_in_a_nested_block",
        |dirs, sandbox| {
            sandbox.with_files(&[Stub::FileWithContent(
                "spam.nu",
                r#"def "foo bar" [] { "from spam" }"#,
            )]);
            let code = r#"def foo [x?] { $"local foo with ($x)" }
def "source spam.nu" [] { "custom" }
do {
    source spam.nu
    foo bar
}"#;
            test()
                .cwd(dirs.test())
                .run(code)
                .expect_value_eq("from spam")
        },
    )
}
