//! Regression tests for the `winnow-parser` front end (calls): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{ast::Expr, engine::StateWorkingSet};
use nu_test_support::prelude::*;
use rstest::rstest;

/// Before `--`, an item starting with `-` is short flags unless it is a negative number for a
/// next positional that takes a number (`parse_short_flags`): these are unknown flags.
#[rstest]
#[case::any_positional("echo -1")]
#[case::list_item("[1 2] | append -1")]
#[case::range("[1 2 3 4] | slice -2..")]
#[case::untyped_parameter("def f [x] { $x }; f -1")]
#[case::not_a_float("def f [x: int] { $x }; f -1_000")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn dash_item_that_is_not_a_negative_number_is_a_flag(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::unknown_flag")
}

/// What `parse_short_flags` makes of an item starting with `-`: a short flag of the command
/// (`-1` here) wins over a number, and `-inf` is a number where the next positional takes one.
#[rstest]
#[case::short_flag_named_like_a_number(
    "def f [--one(-1), x?: int] { {one: $one, x: $x} | to nuon }; f -1",
    "{one: true, x: null}"
)]
#[case::infinity_for_a_number(
    "def --wrapped f [x?: number, ...rest] { $x | describe }; f -inf",
    "float"
)]
#[case::negative_int("def f [x: int] { $x | describe }; f -5", "int")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn dash_item_is_flags_or_a_negative_number(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// An `extern` or `def --wrapped` command takes an item starting with `-` that is none of its
/// flags as an argument, as `parse_short_flags` does: the lowering passes it through, without
/// the classic parser.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn wrapped_command_takes_unknown_dash_items() -> Result {
    let mut tester = test();
    let before = nu_parser::winnow_stats();
    tester
        .run("def --wrapped f [...rest] { $rest }; f -1 -x --y 5")
        .expect_value_eq(["-1", "-x", "--y", "5"])?;
    tester
        .run("def --wrapped g [--one(-1), ...rest] { [$one $rest] | to nuon }; g -1 -2")
        .expect_value_eq(r#"[true, ["-2"]]"#)?;
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "a statement was handed to the classic parser"
    );
    Ok(())
}

/// After an alias of a command, the words that follow may name a subcommand of the aliased
/// command, which is then the command called (`find_longest_decl_with_prefix`).
#[rstest]
#[case::custom_subcommand(
    r#"def foo [x?] { $"foo got ($x)" }; def "foo bar" [] { "sub" }; alias f = foo; f bar"#,
    "sub"
)]
#[case::builtin_subcommand(
    "alias u = update; [[x]; [1] [2]] | u cells {|v| $v + 10} | to nuon",
    "[[x]; [11], [12]]"
)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_followed_by_a_subcommand_calls_the_subcommand(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// `source`, `source-env` and `run` are picked by a statement's first word before any command
/// name is looked up (`parse_builtin_commands`), even when a longer command name, of a `def` or
/// of an alias of an external command, starts with that word: there is no file `me` to read.
#[rstest]
#[case::source(r#"def "source me" [] { "custom" }; source me"#)]
#[case::source_env(r#"def "source-env me" [] { "custom" }; source-env me"#)]
#[case::run(r#"def "run me" [] { "custom" }; run me"#)]
#[case::run_external_alias(r#"alias "run me" = ^echo hi; run me"#)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn first_word_keyword_wins_over_a_longer_command_name(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::sourced_file_not_found")
}

/// `hide me` hides the command `me` even when a command named `hide me` exists, as the classic
/// parser picks `hide` by the statement's first word.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn hide_wins_over_a_longer_command_name() -> Result {
    let code = r#"def me [] { "me" }; def "hide me" [] { "custom" }; hide me
        scope commands | where name == me | length"#;
    test().run(code).expect_value_eq(0)
}

/// The head of an external call is a string or a glob even when it looks like a list or a
/// closure (`parse_external_string`).
#[rstest]
#[case::list("^[a b]")]
#[case::braces("^{a}")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn external_head_is_a_glob(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    let Expr::ExternalCall(head, _) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}` is not an external call");
    };
    assert!(
        matches!(head.expr, Expr::GlobPattern(..)),
        "`{code}`: the head is {:?}",
        head.expr
    );
    Ok(())
}

/// A call the winnow parser read for a `def --wrapped` command, whose name a `hide` has bound
/// to another command since, is read with that command's signature: `0x[ff]` is binary for
/// the built-in `echo`, not an external argument.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn call_to_a_hidden_wrapped_command_uses_the_visible_one() -> Result {
    let code = r#"def --wrapped echo [...rest] { "custom" }; hide echo; echo 0x[ff] | describe"#;
    test().run(code).expect_value_eq("binary")
}

/// A closure that is the last argument of a command taking a row condition (`any`, `all`,
/// `take while`, ...) is lowered as the row condition `parse_row_condition` makes of it, not
/// handed to the classic parser.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn closure_row_condition_is_lowered() -> Result {
    let mut tester = test();
    let before = nu_parser::winnow_stats();
    tester
        .run("[1 2 3] | any {|x| $x == 2 }")
        .expect_value_eq(true)?;
    tester
        .run("[1 2 3] | all { $in > 1 }")
        .expect_value_eq(false)?;
    tester
        .run("[1 2 3 4] | take while --include 1 {|x| $x < 2 } | length")
        .expect_value_eq(2)?;
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "a statement was handed to the classic parser"
    );
    Ok(())
}

/// The arguments of a command taking a row condition (`any`, `take while`, `record where`,
/// or an alias of one) are read as one condition, as `where`'s, and lowered without the
/// classic parser.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn bare_row_condition_is_lowered() -> Result {
    let mut tester = test();
    let before = nu_parser::winnow_stats();
    tester.run("[1 2 3] | any $it > 2").expect_value_eq(true)?;
    tester
        .run("[{a: 1} {a: 5}] | all a > 0")
        .expect_value_eq(true)?;
    tester
        .run("[1 2 3 4] | take while $it < 3 | length")
        .expect_value_eq(2)?;
    tester
        .run("{a: 1, b: 2} | record where $it.value > 1 | columns")
        .expect_value_eq(["b"])?;
    tester
        .run("alias a = any; [1 2 3] | a $it > 2")
        .expect_value_eq(true)?;
    let after = nu_parser::winnow_stats();
    // The `alias` statement goes to the classic parser by design.
    assert_eq!(
        after.classic_statements - before.classic_statements,
        1,
        "a statement was handed to the classic parser"
    );
    Ok(())
}

/// A call through an alias of `match`, `if` or `try` parses as the keyword's statement; only
/// that statement goes to the classic parser, which makes the call with the alias.
#[rstest]
#[case::match_alias(r#"alias m = match; m 1 { 1 => "one", _ => "other" }"#, "one")]
#[case::if_alias(r#"alias i = if; i false { "yes" } else { "no" }"#, "no")]
#[case::try_alias(
    r#"alias t = try; t { error make {msg: x} } catch { "caught" }"#,
    "caught"
)]
#[case::alias_of_the_alias(
    r#"alias m = match; alias n = m; n 2 { 1 => "one", _ => "other" }"#,
    "other"
)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn call_through_an_alias_of_a_keyword(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}
