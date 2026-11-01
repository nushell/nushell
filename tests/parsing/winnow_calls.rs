//! Regression tests for the `winnow-parser` front end (calls): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{ParseError, ast::Expr, engine::StateWorkingSet};
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
/// command, which is then the command called (`find_longest_decl_with_prefix`), by the winnow
/// front end, not handed to the classic parser.
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
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_followed_by_a_subcommand_calls_the_subcommand(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    let mut tester = test();
    let before = nu_parser::winnow_stats();
    tester.run(code).expect_value_eq(expected)?;
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "a statement was handed to the classic parser"
    );
    Ok(())
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
    assert_eq!(
        after.classic_statements, before.classic_statements,
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

/// Parse `code` with the winnow front end, which must lower every statement itself, and give
/// the errors it reported, each as its kind and the text it points at.
pub(super) fn lowered_errors(code: &str) -> Vec<(String, String)> {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    working_set
        .parse_errors
        .iter()
        .map(|error| {
            let debug = format!("{error:?}");
            let kind = debug.split(['(', ' ', '{']).next().unwrap_or_default();
            let text = working_set.get_span_contents(error.span());
            (kind.to_owned(), String::from_utf8_lossy(text).into_owned())
        })
        .collect()
}

/// A call's argument errors are reported as `parse_internal_call` reports them, each where it
/// finds it and with the argument it builds there, and the call goes on: the statement is
/// lowered, not parsed again by the classic parser. A statement whose first word names an
/// alias reports them twice: `parse_builtin_commands` parses it once more before the statement
/// to find an overlay command, and keeps that parse's errors.
#[rstest]
#[case::positional_type("def f [a: int] {}; f {a: 1}", &[("TypeMismatch", "{a: 1}")])]
#[case::leaf_value("def f [a: int] {}; f nope", &[("Expected", "nope")])]
#[case::list_for_an_int("def f [a: int] {}; f [1]", &[("ExpectedWithStringMsg", "[1]")])]
#[case::extra_positional("def f [] {}; f 1", &[("ExtraPositional", "1")])]
#[case::extra_after_end_of_options("def f [] {}; f -- -x", &[("ExtraPositional", "-x")])]
#[case::flag_word_after_end_of_options("def f [a: int] {}; f -- -x", &[("Expected", "-x")])]
#[case::unknown_long_flag("def f [] {}; f --nope", &[("UnknownFlag", "--nope")])]
#[case::unknown_short_flag("def f [--switch(-s)] {}; f -sz", &[("UnknownFlag", "z")])]
#[case::value_flag_not_last(
    "def f [--flag(-f): int, --switch(-s)] {}; f -fs 1",
    &[("OnlyLastFlagInBatchCanTakeArg", "f"), ("ExtraPositional", "1")]
)]
#[case::long_flag_without_value("def f [--flag: int] {}; f --flag", &[("MissingFlagParam", "--flag")])]
#[case::short_flag_without_value("def f [--flag(-f): int] {}; f -f", &[("MissingFlagParam", "-f")])]
#[case::flag_value_type("def f [--flag: int] {}; f --flag {a: 1}", &[("TypeMismatch", "{a: 1}")])]
#[case::short_flag_value("def f [--flag(-f): int] {}; f -f nope", &[("Expected", "nope")])]
#[case::spread_without_rest("def f [] {}; f ...[1]", &[("UnexpectedSpreadArg", "...[1]")])]
#[case::spread_for_a_required("def f [a: int, ...r] {}; f ...[1]", &[("MissingPositional", "")])]
#[case::record_spread_without_flags("def f [] {}; f ...{a: 1}", &[("UnexpectedSpreadArg", "...{a: 1}")])]
#[case::missing_positional("def f [a: int, b: int] {}; f 1", &[("MissingPositional", "")])]
#[case::one_after_another(
    "def f [a: int] {}; f nope 2 --x",
    &[("Expected", "nope"), ("ExtraPositional", "2"), ("UnknownFlag", "--x")]
)]
#[case::in_a_closure(
    "def f [] {}; let c = {|| f 1 }",
    &[("ExtraPositional", "1")]
)]
#[case::statement_through_an_alias(
    "def f [a: int] {}; alias g = f; g nope",
    &[("Expected", "nope"), ("Expected", "nope")]
)]
#[case::nested_statement_through_an_alias(
    "def f [a: int] {}; alias g = f; do { g nope }",
    &[("Expected", "nope"), ("Expected", "nope")]
)]
#[case::element_through_an_alias(
    "def f [a: int] {}; alias g = f; 1 | g nope",
    &[("Expected", "nope")]
)]
#[case::oneof_no_shape_reads_it(
    "def f [x: oneof<int, filesize>] {}; f x",
    &[("ExpectedWithStringMsg", "x")]
)]
#[case::oneof_of_a_builtin("[1 2] | first x", &[("ExpectedWithStringMsg", "x")])]
#[case::oneof_flag_value(
    "def f [--n: oneof<int, filesize>] {}; f --n x",
    &[("ExpectedWithStringMsg", "x")]
)]
#[case::oneof_guess_with_an_error_past_its_start(
    "def f [x: oneof<range, int>] {}; f 1..x",
    &[("Expected", "x")]
)]
#[case::oneof_guess_with_an_error_in_a_closure(
    "def f [x: oneof<int, closure>] {}; f {|| ls --nope }",
    &[("UnknownFlag", "--nope")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn argument_error_is_reported_and_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}

/// An unknown long flag suggests the closest flag the command has, as `parse_long_flag` does.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn unknown_flag_suggests_a_flag() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(
        &mut working_set,
        None,
        b"def f [--zebra] {}; f --zebre",
        false,
    );
    let [ParseError::UnknownFlag(_, name, _, help)] = working_set.parse_errors.as_slice() else {
        panic!("{:?}", working_set.parse_errors);
    };
    assert_eq!(
        (name.as_str(), help.as_str()),
        ("zebre", "Did you mean: `--zebra`?")
    );
    Ok(())
}

/// `alias` and `type` drop the errors of a call for help (`parse_alias`, `parse_type_alias`).
#[rstest]
#[case::alias_short_flags("alias -hx")]
#[case::type_help_value("type t --help=1 = int")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_call_of_alias_or_type_drops_its_errors(#[case] code: &str) -> Result {
    assert_eq!(lowered_errors(code), [], "`{code}`");
    Ok(())
}

/// With an error in its call, a parser keyword's statement is the call as `parse_call` makes
/// it (`parse_keyword`): typed by the signature, even when it asks for help, and without the
/// keyword's effect.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_call_with_an_error_is_the_plain_call() -> Result {
    let code = "overlay new --help={a: 1} o";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(after.classic_statements, before.classic_statements);
    assert!(matches!(
        working_set.parse_errors.as_slice(),
        [ParseError::TypeMismatch(..)]
    ));
    let statement = &block.pipelines[0].elements[0].expr;
    assert_eq!(statement.ty, nu_protocol::Type::Nothing);
    Ok(())
}

/// After `--`, an item that looks like a flag is a positional, read with its shape; one of a
/// multi-item shape (`try`'s `catch` keyword) is read as `parse_multispan_value` reads it.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn flag_word_for_a_multi_item_positional_after_end_of_options() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(
        &mut working_set,
        None,
        b"alias zz = try {} -- catch {} --",
        false,
    );
    assert!(
        matches!(
            working_set.parse_errors.as_slice(),
            [
                ParseError::ExpectedKeyword(..),
                ParseError::KeywordMissingArgument(..)
            ]
        ),
        "{:?}",
        working_set.parse_errors
    );
    Ok(())
}

/// A `oneof` argument no shape reads, with no guess whose error `parse_oneof` keeps, is garbage
/// with an error that lists the shapes in their declared order.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn oneof_argument_no_shape_reads_lists_the_shapes() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let code = b"def f [x: oneof<filesize, int>] {}; f x";
    nu_parser::parse(&mut working_set, None, code, false);
    let [ParseError::ExpectedWithStringMsg(message, _)] = working_set.parse_errors.as_slice()
    else {
        panic!("{:?}", working_set.parse_errors);
    };
    assert_eq!(message, "one of a list of accepted shapes: [Filesize, Int]");
    Ok(())
}

/// A `oneof` argument whose reading by one shape the lowering gives back is the classic
/// parser's: `parse_oneof` chooses by every shape's errors, and that shape reported none.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn oneof_argument_with_a_shape_given_back_is_the_classic_parsers() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let code = b"def f [x: oneof<record, closure>] {}; f {a: $nope}";
    nu_parser::parse(&mut working_set, None, code, false);
    assert!(
        matches!(
            working_set.parse_errors.as_slice(),
            [ParseError::VariableNotFound(..)]
        ),
        "{:?}",
        working_set.parse_errors
    );
    Ok(())
}

/// A variable not in scope (`parse_variable_expr`) and a list's item a leaf parser rejects
/// (`parse_list_expression`) are reported as the classic parser reports them, the value
/// garbage there, and the statement is lowered.
#[rstest]
#[case::variable_argument("echo $nope", &[("VariableNotFound", "$nope")])]
#[case::variable_with_a_cell_path("$nope.a | describe", &[("VariableNotFound", "$nope")])]
#[case::variable_in_a_closure("let c = {|| $nope }", &[("VariableNotFound", "$nope")])]
#[case::variable_in_an_interpolation("$\"($nope)\"", &[("VariableNotFound", "$nope")])]
#[case::list_item(
    "def li [x: list<int>] {}; li [1 nope]",
    &[("Expected", "nope")]
)]
#[case::list_item_of_a_flag_value(
    "def f [--n: list<int>] {}; f --n [1, x]",
    &[("Expected", "x")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn value_error_is_reported_and_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}

/// An environment shorthand whose value has an error is no shorthand (`parse_expression`
/// drops the error): the word is the command, an external one here.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn shorthand_with_an_error_in_its_value_is_the_command() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, b"A=$nope ls", false);
    assert_eq!(working_set.parse_errors, []);
    assert!(matches!(
        block.pipelines[0].elements[0].expr.expr,
        Expr::ExternalCall(..)
    ));
    Ok(())
}

/// What the classic parser reports about a value, an operation or a statement and goes on past
/// is reported as it reports it, and the statement is lowered: a `{ ... }` a shape doesn't take
/// (`parse_brace_expr`, and `parse_oneof` when no shape does), a closure with more parameters,
/// or another type of parameter, than a `closure(...)` shape gives it, a negation or an
/// operation of types it doesn't apply to, a binding's value of another type than written, a
/// constant that doesn't evaluate (also when a module exports it), an attribute that is unknown
/// or doesn't evaluate, a parameter's unknown type or completer, a default value that isn't a
/// constant, an `export alias` in a module asking for help, which nu looks for once parsed, a
/// math operand's leaf error, also at a statement's head when the first item reads as a value,
/// and a row condition that isn't a bool (garbage). The errors of a math expression come in the
/// classic parser's order: an operation's type is checked once its operator stack reduces it,
/// after the operand that follows.
#[rstest]
#[case::non_block_value("def f [r: record] {}; f {$in}", &[("ExpectedWithStringMsg", "{$in}")])]
#[case::no_shape_takes_the_braces(
    "{} | merge {$in}",
    &[("ExpectedWithStringMsg", "{$in}")]
)]
#[case::closure_parameter_count("[1] | each {|a b| $a }", &[("ExpectedWithStringMsg", "|a b|")])]
#[case::closure_parameter_type(
    "[1] | each {|x: string| $x }",
    &[("ParameterMismatchType", "|x: string|")]
)]
#[case::negation("let s = 'a'; -$s", &[("OperatorUnsupportedType", "-")])]
#[case::operation("'a' + 1", &[("OperatorIncompatibleTypes", "+")])]
#[case::operation_after_the_next_operand(
    "1 * 'a' + $nope",
    &[("VariableNotFound", "$nope"), ("OperatorUnsupportedType", "*")]
)]
#[case::let_type("let x: int = 'a'", &[("TypeMismatch", "'a'")])]
#[case::let_value_input(
    "def f []: int -> any { let x = str length; $x }",
    &[("InputMismatch", "str length")]
)]
#[case::const_type(
    "const x: int = 'a'",
    &[("TypeMismatch", "'a'"), ("LabeledError", "'a'")]
)]
#[case::const_value_type(
    "const x: int = [1 'a'] | get 1",
    &[("TypeMismatch", "[1 'a'] | get 1")]
)]
#[case::const_not_constant("const x = (ls)", &[("LabeledError", "(ls)")])]
#[case::exported_const_not_constant(
    "module m { export const x = (ls) }",
    &[("LabeledError", "(ls)"), ("InternalError", "x")]
)]
#[case::attribute_unknown("@nope\ndef f [] {}", &[("UnknownCommand", "nope")])]
#[case::attribute_alias_of_an_external(
    "alias \"attr ext\" = ^ls\n@ext\ndef f [] {}",
    &[("LabeledError", "ext")]
)]
#[case::attribute_not_constant(
    "def \"attr a\" [] { {} }\n@a\ndef f [] {}",
    &[("LabeledError", "a")]
)]
#[case::parameter_type("def f [x: nope] {}", &[("UnknownType", "nope")])]
#[case::parameter_completer("def f [x: string@nope] {}", &[("UnknownCommand", "nope")])]
#[case::parameter_default(
    "def f [x = $nope] {}",
    &[("VariableNotFound", "$nope"), ("NonConstantDefaultValue", "$nope")]
)]
#[case::default_read_with_an_earlier_defaults_type(
    "def f [x = 1..=5] {}",
    &[("Expected", "5"), ("NonConstantDefaultValue", "5")]
)]
#[case::leaf_at_a_statements_head("let y = 1..$.a", &[("OperatorUnsupportedType", "..")])]
#[case::leaf_operand(
    "1 + 1..$.a",
    &[("OperatorUnsupportedType", ".."), ("OperatorUnsupportedType", "+")]
)]
#[case::row_condition_not_a_bool("[1] | where 1", &[("TypeMismatch", "1")])]
#[case::export_alias_help("module m { export alias --help }", &[("InternalError", "alias --help")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn error_gone_past_is_reported_and_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}

/// An `if` asking for help as an operation's operand is a call of `if`, as
/// `parse_math_expression` reads an `if` there (`parse_call`), typed by its blocks: an int added
/// to an int, not to the words of the call read as a string.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn if_help_call_operand_is_a_call() -> Result {
    let code = "1 + if true { 1 } else { 2 } --help";
    assert_eq!(lowered_errors(code), [], "`{code}`");
    Ok(())
}

/// A first item that doesn't read as a value (`is_math_expression_like`) makes the statement a
/// command, an external one, whose argument errors are not reported (`parse_expression`): the
/// range the winnow parser reads, with a variable not in scope, is left to the classic parser.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn first_item_that_is_no_value_is_a_command() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, b"0..<$nope", false);
    assert_eq!(working_set.parse_errors, []);
    assert!(matches!(
        block.pipelines[0].elements[0].expr.expr,
        Expr::ExternalCall(..)
    ));
    Ok(())
}

/// A flag that takes a value takes the item after it, whatever it is, read as `parse_value`
/// reads it with the flag's shape (`parse_long_flag`, `parse_short_flags`): also a word that
/// looks like a flag, a negation, or `--`, which then is no end of the options.
#[rstest]
#[case::flag_word_as_a_string("f --s --x", &[])]
#[case::negation_of_a_string("f --n -$txt", &[("OperatorUnsupportedType", "-")])]
#[case::flag_word_for_a_short_flags_int("f -n --x", &[("Expected", "--x")])]
#[case::end_of_options_for_an_int("f --n --", &[("Expected", "--")])]
#[case::flags_after_a_value_of_two_dashes("f -s -- --n 1", &[])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn flag_value_that_looks_like_a_flag(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let code = format!("def f [--n(-n): int, --s(-s): string] {{}}; let txt = 'a'; {code}");
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(&code), expected, "`{code}`");
    Ok(())
}

/// The arguments of a command that is a parser keyword in nu but a call in the winnow parser
/// (`hide`, `source`, `run`, the `overlay` commands, `plugin use`, the built-in attributes) are
/// checked as any call's are (`parse_internal_call`, `check_call`), and the statement is lowered
/// with nu's errors: the winnow parser checks only the parts that are syntax in nu, the members
/// of `hide` and the `as` of `overlay use`, whose value is any item (`--prefix` names the
/// overlay).
#[rstest]
#[case::hide_extra_positional(
    "hide ls a b",
    &[("ExtraPositional", "b"), ("WrongImportPattern", "b"), ("ModuleNotFound", "ls")]
)]
#[case::hide_help_extra_positional("hide --help a b c", &[("ExtraPositional", "c")])]
#[case::hide_unknown_long_flag("hide ls --foo", &[("UnknownFlag", "--foo")])]
#[case::hide_unknown_short_flag("hide -x ls", &[("UnknownFlag", "x")])]
#[case::hide_negative_number("hide -1 ls", &[("UnknownFlag", "1")])]
#[case::hide_without_argument("hide", &[("MissingPositional", "")])]
#[case::hide_null(
    "hide null",
    &[("ExpectedWithStringMsg", "null"), ("LabeledError", "null"), ("UnknownState", "null")]
)]
#[case::source_two_positionals("source null null", &[("ExtraPositional", "null")])]
#[case::source_unknown_short_flag("source -x null", &[("UnknownFlag", "x")])]
#[case::source_without_argument("source", &[("MissingPositional", "")])]
#[case::source_env_without_argument("source-env", &[("MissingPositional", "")])]
#[case::run_without_argument("run", &[("MissingPositional", "")])]
#[case::run_switch_value_not_bool(
    "run --full-reparse=maybe x.nu",
    &[("ExpectedWithStringMsg", "maybe"), ("LabeledError", "run --full-reparse=maybe x.nu")]
)]
#[case::overlay_new_two_positionals("overlay new a b", &[("ExtraPositional", "b")])]
#[case::overlay_new_without_argument("overlay new", &[("MissingPositional", "")])]
#[case::overlay_new_switch_value_not_bool(
    "overlay new a --reload=3",
    &[("ExpectedWithStringMsg", "3")]
)]
#[case::overlay_hide_two_positionals("overlay hide a b", &[("ExtraPositional", "b")])]
#[case::overlay_hide_flag_without_value("overlay hide --keep-env", &[("MissingFlagParam", "--keep-env")])]
#[case::overlay_hide_value_flag_not_last("overlay hide -ek spam", &[("OnlyLastFlagInBatchCanTakeArg", "e")])]
#[case::overlay_list_positional("overlay list x", &[("ExtraPositional", "x")])]
#[case::overlay_list_unknown_flag("overlay list --foo", &[("UnknownFlag", "--foo")])]
#[case::overlay_use_extra_after_as("overlay use null as x y", &[("ExtraPositional", "y")])]
#[case::overlay_use_without_argument("overlay use", &[("MissingPositional", "")])]
#[case::overlay_use_unknown_flag("overlay use null --foo", &[("UnknownFlag", "--foo")])]
#[case::overlay_use_unknown_flag_in_batch("overlay use -hz spam", &[("UnknownFlag", "z")])]
#[case::overlay_use_as_takes_a_flag("module spam {}\noverlay use spam as --prefix", &[])]
#[case::attribute_extra_positional(
    "@category a b\ndef f [] {}",
    &[("ExtraPositional", "b"), ("LabeledError", "category a b")]
)]
#[case::attribute_without_argument(
    "@example x\ndef f [] {}",
    &[("MissingPositional", ""), ("LabeledError", "example x")]
)]
#[case::attribute_with_a_pipe(
    "@example x | y\ndef f [] {}",
    &[("ExtraPositional", "y"), ("LabeledError", "example x | y")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_command_argument_errors_are_reported_and_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}

/// A call through an alias of `if`, `match` or `try` that the keyword's own parser can't read is
/// read as a call to the alias, as nu reads it: the errors of that call, reported twice as for
/// any statement whose first word names an alias (see [`argument_error_is_reported_and_lowered`]),
/// and no stack overflow, which reading it with the keyword's parser again gave.
#[rstest]
#[case::if_without_arguments("alias i = if; i", &[("MissingPositional", ""), ("MissingPositional", "")])]
#[case::if_without_block("alias i = if; i true", &[("MissingPositional", ""), ("MissingPositional", "")])]
#[case::if_else_without_value(
    "alias i = if; i true {} else",
    &[("KeywordMissingArgument", ""), ("KeywordMissingArgument", "")]
)]
#[case::match_without_block(
    "alias m = match; m 1",
    &[("MissingPositional", ""), ("MissingPositional", "")]
)]
#[case::try_catch_without_handler(
    "alias t = try; t {} catch",
    &[("KeywordMissingArgument", ""), ("KeywordMissingArgument", "")]
)]
#[case::help("alias i = if; i --help", &[])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn failing_call_through_an_alias_of_a_keyword(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}

/// Items nu reads again as one value and reports errors in, with what it recovers: a math
/// expression after a dash item read as an unknown flag, an item where an operator goes, a range
/// bound that is no number, a keyword's argument after `--`, a match block read as a call's
/// argument, an unclosed raw string. All lowered, not handed to the classic parser.
#[rstest]
#[case::if_condition_after_a_flag(
    "if -1 > 0 { }",
    &[("UnknownFlag", "1"), ("Expected", "0"), ("IncompleteMathExpression", "0"), ("IncompleteMathExpression", "0")]
)]
#[case::where_condition_after_a_flag(
    "where -1 > 0",
    &[("UnknownFlag", "1"), ("Expected", "0"), ("IncompleteMathExpression", "0"), ("IncompleteMathExpression", "0")]
)]
#[case::if_as_an_operand(
    "1 + if true { 1 } +",
    &[("Expected", "{ 1 }"), ("IncompleteMathExpression", "{ 1 }"), ("IncompleteMathExpression", "{ 1 }"), ("Expected", "+")]
)]
#[case::match_with_an_extra_argument("match 1 { _ => 1 } extra", &[("ExtraPositional", "extra")])]
#[case::range_bound_string("(1)abc..5", &[("OperatorUnsupportedType", "..")])]
#[case::keyword_after_end_of_options(
    "try {} -- catch {} --",
    &[("ExpectedKeyword", "--"), ("KeywordMissingArgument", "")]
)]
#[case::unclosed_raw_string(
    "r#abc",
    &[("Expected", "a"), ("Unclosed", "r#"), ("Expected", "abc"), ("IncompleteMathExpression", "abc"), ("IncompleteMathExpression", "abc")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn values_read_again_with_errors_are_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(lowered_errors(code), expected, "`{code}`");
    Ok(())
}
