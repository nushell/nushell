//! Regression tests for the `winnow-parser` front end (statements): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{ast::Expr, engine::StateWorkingSet};
use nu_test_support::prelude::*;
use rstest::rstest;

/// `count` statements that each build a closure, about 45 bytes apiece: enough of them make a
/// block long enough to be parsed ahead (`AHEAD_MIN_BYTES` in
/// `crates/nu-parser/src/winnow/driver.rs`).
fn filler(count: usize) -> String {
    (1..=count)
        .map(|i| format!("let x{i} = [1 2 3] | each {{|it| $it + {i} }}\n"))
        .collect()
}

/// nu's lexer turns the end of line before a `|` that starts a line into that `|`, so a `|`
/// ending a line and one starting a later line may have one blank line between them, and
/// comment lines around it.
#[rstest]
#[case::blank_line("[1 2 3] |\n\n| length")]
#[case::comment_then_blank_line("[1 2 3] |\n# c\n\n| length")]
#[case::blank_line_then_comment("[1 2 3] |\n\n# c\n| length")]
#[case::comments_around_blank_line("[1 2 3] |\n# c1\n\n# c2\n| length")]
#[case::same_line_comment("[1 2 3] | # c\n\n| length")]
#[case::crlf("[1 2 3] |\r\n\r\n| length")]
#[case::in_a_definition("def f [] {\n  [1 2 3] |\n\n  | length\n}\nf")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn pipe_continues_across_one_blank_line(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(3)
}

/// After an `e>|` or `o+e>|` that ends a line, nu goes on with the pipeline only through a `|`
/// that starts a later line: the next line is another statement.
#[rstest]
#[case::err_pipe("^echo hi e>|\ndescribe", 2)]
#[case::out_err_pipe("^echo hi o+e>|\ndescribe", 2)]
#[case::comment_line("^echo hi e>|\n# c\ndescribe", 2)]
#[case::line_leading_pipe("^echo hi e>|\n# c\n| describe", 1)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn redirection_pipe_at_the_end_of_a_line(#[case] code: &str, #[case] pipelines: usize) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    assert_eq!(block.pipelines.len(), pipelines, "`{code}`");
    Ok(())
}

/// A definition is a statement of its own, declared before the statements run, when the line
/// before it ends with an `e>|` (nu goes on from there only through a `|` starting a later
/// line), or when a `|` after it is dropped because the end of the block follows: the call
/// above it finds it.
#[rstest]
#[case::after_err_pipe("f\n^echo hi e>|\ndef f [] { 'f ran' }")]
#[case::after_out_err_pipe("f\n^echo hi o+e>|\ndef f [] { 'f ran' }")]
#[case::after_err_pipe_and_comment("f\n^echo hi e>| # c\ndef f [] { 'f ran' }")]
#[case::pipe_at_the_end_of_the_block("f\ndef f [] { 'f ran' } |\n")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn definition_ending_a_pipeline_is_declared(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: the first statement is not a call");
    };
    assert_eq!(Some(call.decl_id), working_set.find_decl(b"f"));
    Ok(())
}

/// nu drops a `|` that a blank line follows: the definition before it is a pipeline of its own,
/// declared before the statements run.
#[rstest]
#[case::blank_line("def f [] { 'f ran' } |\n\nf")]
#[case::same_line_comment("def f [] { 'f ran' } | # c\n\nf")]
#[case::line_leading_pipe("def f [] { 'f ran' }\n|\n\nf")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn definition_with_a_dropped_pipe(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("f ran")
}

/// The same for an `extern`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn extern_with_a_dropped_pipe() -> Result {
    test().run("extern e [] |\n\n'ok'").expect_value_eq("ok")
}

/// nu reads an `@` item that starts a line as an attribute line, also after a `|`: here the
/// pipeline is a lone `|` and the attributed definition.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attribute_line_after_a_pipe() -> Result {
    test()
        .run("|\n@search-terms foo\ndef bar [] { 'bar' }\nbar")
        .expect_value_eq("bar")
}

/// A definition after an attribute line in a longer pipeline is a word of that pipeline's last
/// command, as in nu, so nothing declares it.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attribute_line_inside_a_pipeline_declares_nothing() -> Result {
    let code = "ls | length |\n@search-terms foo\ndef bar [] { 'bar' }";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    assert_eq!(working_set.find_decl(b"bar"), None);
    Ok(())
}

/// A `|` that starts the line after an attribute line goes on with it (nu's lexer turns the end
/// of line into that `|`): the attribute takes `|` and `bar` as arguments.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attribute_line_continues_through_a_line_leading_pipe() -> Result {
    let code = "@search-terms foo\n| bar\ndef baz [] { 1 }\n\
                [(baz) (scope commands | where name == baz | get 0.search_terms)]";
    test()
        .run(code)
        .expect_value_eq(test_list![1, "foo, |, bar"])
}

/// The rest parameter of a `def --wrapped` is its signature's, not a `...word` in a comment or
/// a default value before it: `...rest: string` is typed, so `5` stays an int.
#[rstest]
#[case::comment(
    "def --wrapped f [\n  --level: any # like ...args of git\n  ...rest: string\n] { $level | describe }\nf --level 5"
)]
#[case::default_value(
    "def --wrapped f [--level: any = \"a ...b\", ...rest: string] { $level | describe }\nf --level 5"
)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn wrapped_rest_parameter_is_the_signature_one(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("int")
}

/// A definition is declared under its name's string: `"'echo'"` declares `'echo'`, which does
/// not shadow `echo`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn quoted_definition_name_keeps_its_inner_quotes() -> Result {
    test()
        .run("def --wrapped \"'echo'\" [...rest] { 'quoted echo' }\necho 5 | describe")
        .expect_value_eq("int")
}

/// The same in a block long enough to be parsed ahead, where the name must be as long as the
/// engine's.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn long_quoted_definition_name_in_a_long_block() -> Result {
    let name = format!("{} 'hi'", "a".repeat(70));
    let code = format!("def \"{name}\" [] {{ 'custom' }}\n{}{name}", filler(100));
    test().run(code).expect_value_eq("custom")
}

/// nu-parser lite-parses a whole block before it parses any statement, so a `||` or a
/// redirection error is the block's first error, before those of statements above it.
#[rstest]
#[case::or_or("let x: int = \"a\"\nls || ls", "nu::parser::shell_oror")]
#[case::missing_redirection_target("let x: int = \"a\"\nls o>", "nu::parser::parse_mismatch")]
#[case::two_redirections(
    "let x: int = \"a\"\n%echo a o> b o> c",
    "nu::parser::multiple_redirections"
)]
#[case::in_a_closure("do {\n  let x: int = \"a\"\n  ls || ls\n}", "nu::parser::shell_oror")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn lite_parse_error_comes_first(#[case] code: &str, #[case] error: &str) -> Result {
    test().run(code).expect_error_code_eq(error)
}

/// nu-parser keeps a `|` that no command follows with the last element of its pipeline (it is
/// highlighted), on a later line too and in a statement it parses itself; of a run of them
/// (`| |`), the last goes with the first element of the next statement.
#[rstest]
#[case::later_line("ls\n|\n\nls", [true, false])]
#[case::statement_parsed_by_nu_parser("%echo 1 |\n\n2", [true, false])]
#[case::two_pipes("ls | |\n\nls", [true, true])]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn dangling_pipe_stays_in_the_tree(#[case] code: &str, #[case] pipes: [bool; 2]) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let found: Vec<bool> = block
        .pipelines
        .iter()
        .map(|pipeline| {
            pipeline
                .elements
                .last()
                .is_some_and(|element| element.pipe.is_some())
        })
        .collect();
    assert_eq!(found, pipes, "`{code}`");
    Ok(())
}
