//! Regression tests for the `winnow-parser` front end (statements): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{ParseError, Type, TypeSet, Value, ast::Expr, engine::StateWorkingSet};
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

/// What the body of the alias `s` is.
enum Body {
    /// A call to the command of this name.
    Call(&'static str),
    /// An external call.
    External,
}

/// An `alias` is lowered, not handed to the classic parser, and declared as `parse_alias`
/// declares it. Its body is the text after `=` read as a call with the block's predeclared
/// commands hidden, so a command the block defines later is an external one; a missing argument
/// is forgiven; the description is the alias's doc comment, or `Alias for` the text after `=`.
#[rstest]
#[case::command("alias s = ls -a", Body::Call("ls"), "Alias for `ls -a`")]
#[case::defined_before(
    "def shout [] { 'hi' }\nalias s = shout",
    Body::Call("shout"),
    "Alias for `shout`"
)]
#[case::defined_later(
    "alias s = shout\ndef shout [] { 'hi' }",
    Body::External,
    "Alias for `shout`"
)]
#[case::missing_argument("def f [x] { $x }\nalias s = f", Body::Call("f"), "Alias for `f`")]
#[case::doc_comment(
    "# Lists everything\nalias s = ls -a",
    Body::Call("ls"),
    "Lists everything"
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_is_lowered(#[case] code: &str, #[case] body: Body, #[case] description: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let decl = working_set.get_decl(working_set.find_decl(b"s").expect("`s` is declared"));
    let alias = decl.as_alias().expect("`s` is an alias");
    match (body, &alias.wrapped_call.expr) {
        (Body::Call(name), Expr::Call(call)) => {
            assert_eq!(working_set.get_decl(call.decl_id).name(), name, "`{code}`");
        }
        (Body::External, Expr::ExternalCall(..)) => {}
        (_, other) => panic!("`{code}`: unexpected body {other:?}"),
    }
    assert_eq!(decl.description(), description, "`{code}`");
    Ok(())
}

/// An alias whose body has an error other than a missing argument is garbage, the error kept,
/// as `parse_alias` leaves it: here the undefined variable in the target's block, a statement
/// the classic parser reports.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_with_an_error_in_its_body_is_garbage() -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(
        &mut working_set,
        None,
        b"alias s = if true { $undefined }",
        false,
    );
    assert!(
        matches!(
            working_set.parse_errors.as_slice(),
            [nu_protocol::ParseError::VariableNotFound(..)]
        ),
        "{:?}",
        working_set.parse_errors
    );
    assert!(matches!(
        block.pipelines[0].elements[0].expr.expr,
        Expr::Garbage
    ));
    assert!(working_set.find_decl(b"s").is_none());
    Ok(())
}

/// `alias` and `export alias` in a module's body and `export alias` in a script are lowered:
/// nothing is handed to the classic parser.
#[rstest]
#[case::module_export("module m {\n  export alias s = ls -a\n}\nuse m s")]
#[case::module_private("module m {\n  alias p = ls -a\n  export alias s = p\n}\nuse m s")]
#[case::script_export("export alias s = ls -a")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_in_a_module_or_exported_is_lowered(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let decl = working_set.get_decl(working_set.find_decl(b"s").expect("`s` is declared"));
    assert!(decl.as_alias().is_some(), "`{code}`: `s` is an alias");
    Ok(())
}

/// The module the `use` tests import from.
const USE_MODULE: &str =
    "module m { export def f [] { 1 }; export def g [] { 2 }; export const c = 3 }\n";

/// `use`, and `export use` in a script or a module's body, are lowered, not handed to the
/// classic parser: the call to `use`, then `finish_use` as `parse_use` does it, which brings the
/// members into scope (`declared`).
#[rstest]
#[case::name("use m f", Some("f"))]
#[case::glob("use m *", Some("g"))]
#[case::list("use m [f g]", Some("g"))]
#[case::whole_module("use m", Some("m f"))]
#[case::end_of_options("use -- m f", Some("f"))]
#[case::constant("use m c\n$c", None)]
#[case::null("use null", None)]
#[case::script_export("export use m f", Some("f"))]
#[case::module_body("module n { use m f; export def h [] { f } }\nuse n h", Some("h"))]
#[case::module_export("module n { export use m [f c] }\nuse n *\n$c", Some("f"))]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn use_is_lowered(#[case] code: &str, #[case] declared: Option<&str>) -> Result {
    let code = format!("{USE_MODULE}{code}");
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    if let Some(name) = declared {
        assert!(
            working_set.find_decl(name.as_bytes()).is_some(),
            "`{code}`: `{name}` is not declared"
        );
    }
    Ok(())
}

/// What `finish_use` reports is kept with the lowered statement, never handed back: the
/// classic parser would load the module and bring its members into scope a second time. Here
/// a module that does not exist, and a second `--`, which is a member, not the end of the
/// options (`parse_internal_call` takes only the first).
#[rstest]
#[case::missing_module("use nope", "ModuleNotFound", 1)]
#[case::second_end_of_options("use m -- --", "ExportNotFound", 1)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn use_keeps_the_errors_of_finish_use(
    #[case] code: &str,
    #[case] error: &str,
    #[case] count: usize,
) -> Result {
    let code = format!("{USE_MODULE}{code}");
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        errors.len() == count && errors.iter().all(|e| e.starts_with(error)),
        "`{code}`: {errors:?}"
    );
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    Ok(())
}

/// The call to `use` gets the library directories, `$NU_LIB_DIRS`, which `parse_internal_call`
/// hands to `use`, `overlay use`, `source-env` and `nu-check`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn use_call_gets_the_library_directories() -> Result {
    let code = format!("const NU_LIB_DIRS = []\n{USE_MODULE}use m f");
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    let use_statement = &block.pipelines[2].elements[0].expr.expr;
    let Expr::Call(call) = use_statement else {
        panic!("`{code}`: not a call: {use_statement:?}");
    };
    assert!(
        call.get_parser_info(nu_engine::DIR_VAR_PARSER_INFO)
            .is_some(),
        "`{code}`: no library directories"
    );
    Ok(())
}

/// `module`, and `export module` in a script or a module's body, are lowered, not handed to
/// the classic parser: the call to `module`, then `finish_module` as `parse_module` does it,
/// which parses the body as the module's block and adds the module. Nothing is handed over:
/// the `use` statements are lowered too.
#[rstest]
#[case::inline("module m { export def f [] { 1 } }\nuse m f", "f")]
#[case::script_export("export module m { export def f [] { 1 } }\nuse m f", "f")]
#[case::private_submodule(
    "module a {\n  module b { export def c [] {} }\n  export use b c\n}\nuse a c",
    "c"
)]
#[case::exported_submodule("module a { export module b { export def c [] {} } }\nuse a b c", "c")]
#[case::mod_submodule("module a { export module mod { export def x [] {} } }\nuse a x", "x")]
#[case::end_of_options("module m { export def f [] {} } --\nuse m f", "f")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_is_lowered(#[case] code: &str, #[case] declared: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    assert!(
        working_set.find_decl(declared.as_bytes()).is_some(),
        "`{code}`: `{declared}` is not declared"
    );
    Ok(())
}

/// The comments before a `module` statement document the module, with those at the top of its
/// body (`finish_module`).
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_doc_comments_are_kept() -> Result {
    let code = "# Before\nmodule m {\n  # Inside\n\n  export def f [] {}\n}";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    let module_id = working_set.find_module(b"m").expect("`m` is a module");
    let comments: Vec<String> = working_set
        .get_module_comments(module_id)
        .unwrap_or_default()
        .iter()
        .map(|span| String::from_utf8_lossy(working_set.get_span_contents(*span)).into_owned())
        .collect();
    assert_eq!(comments, ["# Before", "# Inside"], "`{code}`");
    Ok(())
}

/// What `finish_module` reports is kept with the lowered statement, never handed back, as
/// nu-parser reports it: a module file that does not exist, and an `export module` named as the
/// module it is in.
#[rstest]
#[case::missing_file("module nope.nu", &["ModuleNotFound"])]
#[case::named_as_parent("module a { export module a {} }", &["NamedAsModule"])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_keeps_the_errors_of_finish_module(
    #[case] code: &str,
    #[case] expected: &[&str],
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        errors.len() == expected.len()
            && errors
                .iter()
                .zip(expected)
                .all(|(e, name)| e.starts_with(name)),
        "`{code}`: {errors:?}"
    );
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    Ok(())
}

/// The call to `module` covers the `--` nu drops after the body, as `parse_module`'s covers all
/// the statement's items.
#[rstest]
#[case::module("module m {} --")]
#[case::export("export module m {} --")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_call_covers_its_end_of_options(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    let span = block.pipelines[0].elements[0].expr.span;
    assert_eq!(working_set.get_span_contents(span), code.as_bytes());
    Ok(())
}

/// What a parser keyword did to the working set, which `parser_keyword_is_lowered` checks.
#[derive(Debug)]
enum Effect {
    /// The last statement is a call marked as a no-op (`source null`, `run null`).
    Noop,
    /// The command is declared.
    Declared(&'static str),
    /// The command is not declared.
    Hidden(&'static str),
    /// The overlay exists.
    Overlay(&'static str),
}

/// The parser keywords other than the module statements are lowered, not handed to the classic
/// parser, and do what the classic parser makes them do: `source`, `source-env` and `run` (of
/// `null`, which loads nothing), `hide`, and the overlay commands, also through an alias.
#[rstest]
#[case::source_null("source null", Effect::Noop)]
#[case::source_env_null("source-env null", Effect::Noop)]
#[case::run_null("run null a --x=1 ...[1] -- --y=2", Effect::Noop)]
#[case::hide("def f [] {}\nhide f", Effect::Hidden("f"))]
#[case::hide_members(
    "module m { export def a [] {}; export def b [] {} }\nuse m\nhide m [a b]",
    Effect::Hidden("m b")
)]
#[case::overlay_use_as(
    "module m { export def a [] {} }\noverlay use m as n --prefix",
    Effect::Declared("n a")
)]
#[case::overlay_new("overlay new o", Effect::Overlay("o"))]
#[case::overlay_alias(
    "module m { export def a [] {} }\nalias ou = overlay use\nou m",
    Effect::Declared("a")
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn parser_keyword_is_lowered(#[case] code: &str, #[case] effect: Effect) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    match effect {
        Effect::Noop => {
            let last = block
                .pipelines
                .last()
                .map(|pipeline| &pipeline.elements[0].expr.expr);
            assert!(
                matches!(last, Some(Expr::Call(call)) if call.get_parser_info("noop").is_some()),
                "`{code}`: {last:?}"
            );
        }
        Effect::Declared(name) => assert!(
            working_set.find_decl(name.as_bytes()).is_some(),
            "`{code}`: `{name}` is not declared"
        ),
        Effect::Hidden(name) => assert!(
            working_set.find_decl(name.as_bytes()).is_none(),
            "`{code}`: `{name}` is declared"
        ),
        Effect::Overlay(name) => assert!(
            working_set.find_overlay(name.as_bytes()).is_some(),
            "`{code}`: no overlay `{name}`"
        ),
    }
    Ok(())
}

/// `%name` calls the built-in command even where a custom command shadows its name, its
/// arguments read as a call to that command reads them (a row condition for `take while`), and
/// `%$name` calls the built-in an expression names at run time, each argument a word or value.
#[rstest]
#[case::shadowed("def length [] { 'mine' }\n[1 2 3] | %length", Value::test_int(3))]
#[case::row_condition("[1 2 3] | %take while $it < 3 | length", Value::test_int(2))]
#[case::dynamic("let c = 'length'\n[1 2 3] | %$c", Value::test_int(3))]
#[case::dynamic_flag(
    "let c = 'echo'\n%$c --x -y | to nuon",
    Value::test_string("[--x, -y]")
)]
#[case::dynamic_spread("let c = 'echo'\n%($c) 1 ...[2 3] | length", Value::test_int(3))]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn percent_call_runs_the_built_in(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// `%` calls are lowered, not handed to the classic parser. As there, the head of `% ls` starts
/// past the `%`, so it covers the blank before the name.
#[rstest]
#[case::bare("%ls", "ls")]
#[case::lone_sigil("% ls", " ls")]
#[case::row_condition("[1 2 3] | %take while $it < 3", "take while")]
#[case::shadowed("def ls [] {}\n%ls", "ls")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn percent_call_is_lowered(#[case] code: &str, #[case] head: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let last = block
        .pipelines
        .last()
        .and_then(|pipeline| pipeline.elements.last());
    let Some(Expr::Call(call)) = last.map(|element| &element.expr.expr) else {
        panic!("`{code}`: not a call: {last:?}");
    };
    assert_eq!(working_set.get_span_contents(call.head), head.as_bytes());
    Ok(())
}

/// `type`, and `export type` in a script or a module's body, are lowered, not handed to the
/// classic parser: `finish_type_alias` defines the alias as `parse_type_alias` does, and an
/// export is a call to `export type`. A type nu can't read defines the alias as `any`, with
/// the error kept.
#[rstest]
#[case::script("type T = record<x: int>", "type", &[])]
#[case::script_export("export type T = list<int>", "export type", &[])]
#[case::module("module m { type T = int; export def f [x: T] {} }\nuse m f\ntype T = int", "type", &[])]
#[case::module_export("module m { export type T = int }\nuse m T", "use", &[])]
#[case::unknown_type("type T = Foo", "type", &["UnknownType"])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn type_alias_is_lowered(
    #[case] code: &str,
    #[case] command: &str,
    #[case] expected: &[&str],
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        errors.len() == expected.len()
            && errors
                .iter()
                .zip(expected)
                .all(|(e, name)| e.starts_with(name)),
        "`{code}`: {errors:?}"
    );
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    assert!(
        working_set.find_type_alias(b"T").is_some(),
        "`{code}`: no type alias `T`"
    );
    let last = &block.pipelines.last().expect("a statement").elements[0]
        .expr
        .expr;
    let Expr::Call(call) = last else {
        panic!("`{code}`: not a call: {last:?}");
    };
    assert_eq!(
        working_set.get_decl(call.decl_id).name(),
        command,
        "`{code}`"
    );
    Ok(())
}

/// What a statement keyword's call for help leaves, which `help_call_is_lowered` checks.
#[derive(Debug)]
enum Help {
    /// The last statement is a call to this command with this many arguments.
    Call(&'static str, usize),
    /// The last statement is a call to this command, typed `any`.
    Untyped(&'static str),
    /// The command is declared: an `extern` defines it even when it asks for help.
    Declared(&'static str),
    /// The command is not declared: a `def` that asks for help defines nothing.
    NotDeclared(&'static str),
}

/// A statement keyword's call for help is lowered, not handed to the classic parser, and is the
/// call the keyword's parse function makes: `module` reads only its name and leaves its body
/// garbage, `export-env` reads only its first item, `type` its first two, an `export` makes it
/// a call to `export <keyword>`, and any `overlay` call for help is typed `any`.
#[rstest]
#[case::def("def f --help [] {}", Help::NotDeclared("f"))]
#[case::def_after_signature("def f [x: int] {} --help", Help::NotDeclared("f"))]
#[case::extern_def("extern e --help []", Help::Declared("e"))]
#[case::export_extern_in_module(
    "module m { export extern e --help [] }\nuse m e",
    Help::Declared("e")
)]
#[case::export_def("export def f --help [] {}", Help::Call("export def", 4))]
#[case::module("module m --help {}", Help::Call("module", 3))]
#[case::export_env("export-env --help extra {}", Help::Call("export-env", 1))]
#[case::type_alias("type t --help = int", Help::Call("type", 2))]
#[case::for_loop("for x --help in [1] {}", Help::Call("for", 4))]
#[case::let_binding("let x --help", Help::Call("let", 2))]
#[case::where_condition("where -h true", Help::Call("where", 2))]
#[case::overlay_list("overlay list --help", Help::Untyped("overlay list"))]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_call_is_lowered(#[case] code: &str, #[case] help: Help) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let last = &block.pipelines.last().expect("a statement").elements[0].expr;
    let call = match &last.expr {
        Expr::Call(call) => call,
        other => panic!("`{code}`: not a call: {other:?}"),
    };
    let name = working_set.get_decl(call.decl_id).name();
    match help {
        Help::Call(command, arguments) => {
            assert_eq!(name, command, "`{code}`");
            assert_eq!(call.arguments.len(), arguments, "`{code}`: {call:?}");
        }
        Help::Untyped(command) => {
            assert_eq!(name, command, "`{code}`");
            assert_eq!(last.ty, Type::Any, "`{code}`");
        }
        Help::Declared(command) => assert!(
            working_set.find_decl(command.as_bytes()).is_some(),
            "`{code}`: `{command}` is not declared"
        ),
        Help::NotDeclared(command) => assert!(
            working_set.find_decl(command.as_bytes()).is_none(),
            "`{code}`: `{command}` is declared"
        ),
    }
    Ok(())
}

/// `let` that asks for help gives the statement's input type, once it has read its variable.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_call_of_let_has_the_input_type() -> Result {
    let code = "def f []: int -> int { let x --help }";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    let decl_id = working_set.find_decl(b"f").expect("`f` is declared");
    let block_id = working_set
        .get_decl(decl_id)
        .block_id()
        .expect("`f` has a body");
    let body = working_set.get_block(block_id);
    assert_eq!(body.pipelines[0].elements[0].expr.ty, Type::Int);
    Ok(())
}

/// A redirected call for help of a statement keyword is an error, as the classic parser
/// reports it, and a `def` that asks for help in a block is still predeclared from its text:
/// the block adds the command's declaration.
#[rstest]
#[case::redirected_def("def f --help o> /dev/null", 1, 0)]
#[case::redirected_module("module m --help {} e> /dev/null", 1, 0)]
#[case::def_in_a_block("do { def f [] {} --help }", 0, 1)]
#[case::export_alias_in_module("module m { export alias --help }", 1, 0)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_call_keeps_the_classic_effects(
    #[case] code: &str,
    #[case] errors: usize,
    #[case] decls: usize,
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors.len(), errors, "`{code}`");
    assert_eq!(working_set.delta.num_decls(), decls, "`{code}`");
    Ok(())
}

/// The body of a `def` that asks for help is compiled once, by `finish_def`, as for any `def`:
/// an error its compilation reports is reported once.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_call_of_def_compiles_its_body_once() -> Result {
    let code = "def f --help [] { break }";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(working_set.compile_errors.len(), 1, "`{code}`");
    Ok(())
}

/// `export-env` reads only its first item, so a `--help` after the block is not read: the
/// statement is the block's.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn export_env_reads_only_its_first_item() -> Result {
    let code = "export-env {} --help";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: not a call");
    };
    assert_eq!(call.arguments.len(), 1, "`{code}`: {call:?}");
    assert!(call.get_named_arg("help").is_none(), "`{code}`: {call:?}");
    Ok(())
}

/// Where the classic parser reads a command (in a pipeline, after an environment shorthand),
/// keywords are lowered as `parse_expression` reads them: `run` is `parse_run_expr`,
/// `source-env`, `let` and `overlay list` are ordinary calls, and so is `plugin use` after a
/// shorthand. The last element's type is checked: `let` gives its input.
#[rstest]
#[case::run_last("[1 2] | run null", "run")]
#[case::run_after_shorthand("FOO=1 run null", "with-env")]
#[case::source_env("[1 2] | source-env null", "source-env")]
#[case::let_last("[1 2] | let x", "let")]
#[case::let_after_shorthand("FOO=1 let x", "with-env")]
#[case::overlay_list("null | overlay list", "overlay list")]
#[case::plugin_use_after_shorthand("FOO=1 plugin use x", "with-env")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_in_a_pipeline_is_lowered(#[case] code: &str, #[case] command: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let last = &block.pipelines[0].elements.last().expect("an element").expr;
    let Expr::Call(call) = &last.expr else {
        panic!("`{code}`: not a call: {last:?}");
    };
    assert_eq!(
        working_set.get_decl(call.decl_id).name(),
        command,
        "`{code}`"
    );
    if command == "let" {
        assert_eq!(last.ty, Type::List(Box::new(Type::Int)), "`{code}`");
    }
    Ok(())
}

/// A `run` before the end of a pipeline is left to the classic parser: once `finish_run` has
/// parsed the script, a later element handed back would hand the whole statement back.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn run_before_the_end_of_a_pipeline_is_handed_over() -> Result {
    let code = "run null | length";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(after.classic_statements, before.classic_statements + 1);
    Ok(())
}

/// A value in the place of a keyword's block, `match`'s arms or `try`'s handler is lowered, the
/// value itself the argument, as the classic parser reads it: the call is typed as
/// `parse_internal_call` types it, `if` by its branches that are blocks, `match` by its arms'
/// value, and an empty `{}` for `match`'s arms is a match block of no arms.
#[rstest]
#[case::if_value("if true $env.FOO", Type::Any)]
#[case::if_value_else_block("if true $env.FOO else { 1 }", Type::Int)]
#[case::loop_value("loop $env.FOO", Type::Nothing)]
#[case::try_subexpression("try ($env.FOO)", Type::Any)]
#[case::match_closure("match 1 {|x| 1}", Type::Closure)]
#[case::match_help_empty_arms("match 1 --help {}", Type::Nothing)]
#[case::try_handler_value("let h = {|e| 1}\ntry { 1 } catch $h", Type::Any)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn value_in_a_block_place_is_lowered(#[case] code: &str, #[case] ty: Type) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let last = &block.pipelines.last().expect("a statement").elements[0].expr;
    assert_eq!(last.ty, ty, "`{code}`");
    Ok(())
}

/// A value in a handler's place must be typed as a closure (`parse_internal_call`'s check of
/// `try`'s positional).
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn handler_value_that_is_no_closure_is_an_error() -> Result {
    let code = "let x = {a: 1}\ntry { 1 } catch $x";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert!(
        matches!(
            working_set.parse_errors.as_slice(),
            [nu_protocol::ParseError::TypeMismatch(..)]
        ),
        "`{code}`: {:?}",
        working_set.parse_errors
    );
    Ok(())
}

/// In a pipeline or after an environment shorthand, `let` and a name is a call to a command of
/// that name when one exists, as `parse_call` looks for the longest command name.
#[rstest]
#[case::pipeline(r#"def "let me" [] { "custom" }; [1] | let me"#)]
#[case::after_shorthand(r#"def "let me" [] { "custom" }; FOO=1 let me"#)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn let_in_a_pipeline_calls_a_longer_command(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    test().run(code).expect_value_eq("custom")
}

/// A call through an alias of `if`, `match` or `try` is lowered as `parse_internal_call` reads
/// an alias's call: a call to the keyword with the alias's name as its head, typed by the
/// signature (the branches' and arms' types don't override it), and its blocks get no input.
#[rstest]
#[case::if_else("alias i = if\ni true { 1 } else { 2 }", "if", "i")]
#[case::if_piped("alias i = if\n'str' | i true { $in }", "if", "i")]
#[case::alias_of_alias("alias i = if\nalias j = i\nj true { 1 }", "if", "j")]
#[case::match_arms("alias m = match\nm 1 { 1 => 'one', _ => 'other' }", "match", "m")]
#[case::match_piped("alias m = match\n'str' | m 1 { _ => $in }", "match", "m")]
#[case::try_catch("alias t = try\nt { 1 } catch { 2 }", "try", "t")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn call_through_an_alias_of_a_keyword_is_lowered(
    #[case] code: &str,
    #[case] keyword: &str,
    #[case] alias: &str,
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    let last = &block.pipelines.last().expect("a statement").elements;
    let last = &last.last().expect("an element").expr;
    let Expr::Call(call) = &last.expr else {
        panic!("`{code}`: not a call: {last:?}");
    };
    assert_eq!(
        working_set.get_decl(call.decl_id).name(),
        keyword,
        "`{code}`"
    );
    assert_eq!(working_set.get_span_contents(call.head), alias.as_bytes());
    assert_eq!(last.ty, Type::Any, "`{code}`");
    // A block after the condition, or `match`'s arms, giving `$in`: typed without the
    // pipeline's input, a string.
    if code.contains("$in") {
        let branch = call.positional_iter().nth(1).expect("a second positional");
        let branch_type = match &branch.expr {
            Expr::Block(block_id) => working_set.get_block(*block_id).output_type(),
            _ => branch.ty.clone(),
        };
        assert_eq!(branch_type, Type::Any, "`{code}`");
    }
    Ok(())
}

/// Parses `code` with the winnow front end and checks that it reported no error and handed no
/// statement to the classic parser. Returns the working set and the block.
fn parse_lowered<'e>(
    engine_state: &'e nu_protocol::engine::EngineState,
    code: &str,
) -> (StateWorkingSet<'e>, std::sync::Arc<nu_protocol::ast::Block>) {
    let mut working_set = StateWorkingSet::new(engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    (working_set, block)
}

/// A positional that reads several items (`parse_multispan_value`) is lowered when the items
/// `calculate_end_span` gives it are read one at a time: a condition of one item, a keyword
/// and its value, a `oneof` of keywords, and `if` or `match` with the items after it, a call.
/// The call is typed as `parse_internal_call` types `if`: its blocks get the input, its output
/// is the branches' union, with `nothing` when it has fewer than three arguments, flags
/// counted (`if --help=false true { 1 }` has three). Without an `else`, the condition takes
/// every item but the block's (`if true { 1 } --help` is an error).
#[rstest]
#[case::if_help("if true --help", "if", Type::Any)]
#[case::while_help("while true --help", "while", Type::Nothing)]
#[case::flag_counts_as_argument("if --help=false true { 1 }", "if", Type::Int)]
#[case::else_then_help("if true { 1 } else { 's' } --help", "if", Type::Int.union(Type::String))]
#[case::if_input("'s' | if true { $in } else { $in } --help", "if", Type::String)]
#[case::try_handler_then_end_of_options("try {} --help catch {} --", "try", Type::Any)]
#[case::if_as_condition("if if --help --help", "if", Type::Any)]
#[case::condition_of_several_items("if 1 > 0 { 1 } else { 2 } --help", "if", Type::Int)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn multi_item_argument_is_lowered(
    #[case] code: &str,
    #[case] keyword: &str,
    #[case] ty: Type,
) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let last = &block.pipelines[0].elements.last().expect("an element").expr;
    let Expr::Call(call) = &last.expr else {
        panic!("`{code}`: not a call: {last:?}");
    };
    assert_eq!(
        working_set.get_decl(call.decl_id).name(),
        keyword,
        "`{code}`"
    );
    assert!(call.get_named_arg("help").is_some(), "`{code}`: {call:?}");
    assert_eq!(last.ty, ty, "`{code}`");
    match code {
        "try {} --help catch {} --" => assert!(
            matches!(
                &call.positional_iter().nth(1).expect("a handler").expr,
                Expr::Keyword(_)
            ),
            "`{code}`: {call:?}"
        ),
        "if if --help --help" => {
            let Expr::Call(condition) = &call.positional_iter().next().expect("a condition").expr
            else {
                panic!("`{code}`: the condition is not a call: {call:?}");
            };
            assert!(condition.get_named_arg("help").is_some(), "`{code}`");
        }
        _ => {}
    }
    Ok(())
}

/// A `break` in a handler of a call for help of `try` in a loop is no error: as in a `try`
/// statement, `parse_internal_call` drops the handler's own `NotInALoop`.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn break_in_a_help_call_handler_compiles() -> Result {
    let code = "loop { try {} --help catch { break } }";
    let engine_state = test().engine_state;
    let (working_set, _) = parse_lowered(&engine_state, code);
    assert_eq!(working_set.compile_errors, [], "`{code}`");
    Ok(())
}

/// `let` without a value is a call to `let` (`parse_let` with fewer than four items): its
/// variable, declared as written (a `$` before the name kept), exists afterwards, after a `--`
/// too, and at statement level a command `let me` is not called.
#[rstest]
#[case::name("let input\n$input | describe", "input", "nothing")]
#[case::end_of_options("let -- x\n$x | describe", "x", "nothing")]
#[case::dollar("let $x\n$x | describe", "$x", "nothing")]
#[case::piped("'s' | let y\n$y | describe", "y", "string")]
#[case::longer_command_name(
    "def \"let me\" [] { 'custom' }\nlet me\n$me | describe",
    "me",
    "nothing"
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn let_without_a_value_is_lowered(
    #[case] code: &str,
    #[case] name: &str,
    #[case] expected: &str,
) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let statement = &block.pipelines[block.pipelines.len() - 2].elements;
    let Expr::Call(call) = &statement.last().expect("an element").expr.expr else {
        panic!("`{code}`: not a call");
    };
    let variable = call.positional_iter().next().expect("a variable");
    assert!(matches!(variable.expr, Expr::VarDecl(_)), "`{code}`");
    assert_eq!(
        working_set.get_span_contents(variable.span),
        name.as_bytes()
    );
    test().run(code).expect_value_eq(expected)
}

/// `mut` or `const` without a value is a call to the keyword too (`parse_mut` and
/// `parse_const` without an `=`), whose missing value is reported as `parse_internal_call`
/// reports it, after a `--` or a `$` name as well. An `export const` without a value, which
/// holds nothing to export, adds no error of its own in a module.
#[rstest]
#[case::const_name("const x")]
#[case::mut_name("mut x")]
#[case::end_of_options("const -- x")]
#[case::dollar("mut $x")]
#[case::in_a_block("do { const x }")]
#[case::export("export const x")]
#[case::module_export("module m { export const x }")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn binding_without_a_value_is_lowered(#[case] code: &str) -> Result {
    let expected = [("MissingPositional".to_owned(), String::new())];
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A `let` in a longer pipeline is an ordinary call, redirected as any, and a `const` without a
/// value ignores its redirection (`parse_const` reads the command's parts alone): lowered, with
/// the errors nu reports.
#[rstest]
#[case::let_before_an_error_pipe("let x e>| length", &[])]
#[case::let_after_a_pipe("[1] | let x o> f", &[])]
#[case::let_with_both_streams("ls | let x o> f e> g", &[])]
#[case::const_help("const x --help o> f", &[])]
#[case::const_without_a_value("const x o> f", &[("MissingPositional", "")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn binding_redirection_is_lowered(#[case] code: &str, #[case] expected: &[(&str, &str)]) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// The value of a keyword's `--help=value` is the switch's `bool`, read as any call's flag
/// value: a value of another type is reported where nu reports it, and the call for help is
/// lowered; `alias` and `type` drop the error with the call's others.
#[rstest]
#[case::if_keyword("if --help=1 true { 1 }", &[("ExpectedWithStringMsg", "1")])]
#[case::def("def --help=abc", &[("ExpectedWithStringMsg", "abc")])]
#[case::module("module x --help=[1]", &[("ExpectedWithStringMsg", "[1]")])]
#[case::for_loop("for --help=1", &[("ExpectedWithStringMsg", "1")])]
#[case::let_binding("let x --help=1", &[("ExpectedWithStringMsg", "1")])]
#[case::variable("return --help=$nope", &[("VariableNotFound", "$nope")])]
#[case::alias("alias --help=1", &[])]
#[case::type_alias("type --help=1", &[])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn help_flag_value_is_lowered(#[case] code: &str, #[case] expected: &[(&str, &str)]) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A `def` or `extern` whose call to the keyword has an error of its own (an unknown flag, a
/// flag value that is no `bool`, a flag where the body goes, a `--wrapped` without a rest, a
/// command's own `--help`) is lowered with nu's errors: `parse_def` and `parse_extern` read the
/// statement with `parse_internal_call` and finish any call.
#[rstest]
#[case::unknown_flag("def --foo f [] { 1 }", &[("UnknownFlag", "--foo")])]
#[case::short_flag("def -1 f [] { 1 }", &[("UnknownFlag", "1")])]
#[case::flag_after_the_name("def f --foo [] { 1 }", &[("UnknownFlag", "--foo")])]
#[case::flag_in_the_body_place("def f [] { 1 } --env", &[("MissingPositional", "")])]
#[case::end_of_options_in_the_body_place("def f [] { 1 } --", &[("MissingPositional", "")])]
#[case::wrapped_without_rest("def --wrapped f [] { 1 }", &[("MissingPositional", "f")])]
#[case::own_help_flag("def f [--help] { 1 }", &[("LabeledErrorWithHelp", "--help")])]
#[case::extern_unknown_flag("extern --foo e []", &[("UnknownFlag", "--foo")])]
#[case::flag_value(
    "def --env=1 f [] { 1 }",
    &[("ExpectedWithStringMsg", "1"), ("LabeledError", "def --env=1 f [] { 1 }")]
)]
#[case::flag_value_before_a_body_error(
    "def --env=1 f [] { $no_such_variable }",
    &[
        ("ExpectedWithStringMsg", "1"),
        ("VariableNotFound", "$no_such_variable"),
        ("LabeledError", "def --env=1 f [] { $no_such_variable }"),
    ]
)]
#[case::exported_flag_value(
    "export def --env=1 f [] { 1 }",
    &[
        ("ExpectedWithStringMsg", "1"),
        ("LabeledError", "export def --env=1 f [] { 1 }"),
        ("InternalError", "def --env=1 f [] { 1 }"),
    ]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn definition_call_error_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A `def` or `extern` with an unknown flag still defines its command, with its description.
#[rstest]
#[case::def("# doc\ndef --foo f [] { 1 }", b"f")]
#[case::extern_command("# doc\nextern --foo e []", b"e")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn definition_with_an_unknown_flag_defines_the_command(
    #[case] code: &str,
    #[case] name: &[u8],
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`"
    );
    let decl_id = working_set.find_decl(name).expect("the command is defined");
    assert_eq!(
        working_set.get_decl(decl_id).description(),
        "doc",
        "`{code}`"
    );
    Ok(())
}

/// A statement keyword whose call has an error of its own (a flag the keyword does not have
/// at a positional boundary, a positional too many) is nu's call to the keyword
/// (`parse_internal_call`): lowered, with the errors nu reports.
#[rstest]
#[case::return_flag("return --foo", &[("UnknownFlag", "--foo")])]
#[case::loop_flag("loop --foo {}", &[("UnknownFlag", "--foo")])]
#[case::loop_extra("loop {} extra", &[("ExtraPositional", "extra")])]
#[case::break_extra("break x", &[("ExtraPositional", "x")])]
#[case::if_flag("if --foo true { 1 }", &[("UnknownFlag", "--foo")])]
#[case::try_trailing_flag("try {} catch {} --foo", &[("UnknownFlag", "--foo")])]
#[case::where_in_a_pipeline("[1] | where --foo $it > 0", &[("UnknownFlag", "--foo")])]
#[case::let_flag("let x --foo", &[("UnknownFlag", "--foo")])]
#[case::const_flag(
    "const --foo x",
    &[("UnknownFlag", "--foo"), ("MissingPositional", "")]
)]
#[case::export_env_flag(
    "export-env --foo {}",
    &[("UnknownFlag", "--foo"), ("MissingPositional", "")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_call_error_is_lowered(#[case] code: &str, #[case] expected: &[(&str, &str)]) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A malformed literal, wherever a value goes, and a bare word with a `:` in a record are
/// errors the winnow parser goes on past: the statement is lowered, the item read again by
/// nu's leaf parser, which reports nu's error with its garbage.
#[rstest]
#[case::statement_head("2023-02-30", &[("InvalidLiteral", "2023-02-30")])]
#[case::call_argument("echo 0b2", &[("InvalidLiteral", "0b2")])]
#[case::list_item("[0x[zz]]", &[("InvalidBinaryString", "0x[zz]")])]
#[case::record_value("{a: 8EiB}", &[("LiteralTooLarge", "8EiB")])]
#[case::math_operand("1 + 0o9", &[("InvalidLiteral", "0o9")])]
#[case::record_colon("{a: x:y}", &[("InvalidLiteral", ":")])]
#[case::binding_value(r#"let x = "\q""#, &[("InvalidLiteral", r#"\q""#)])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn malformed_literal_is_lowered(#[case] code: &str, #[case] expected: &[(&str, &str)]) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A reserved variable name (`in`, `nu`, `env`, `ans`, a `$` before it or not) where a variable
/// is declared is nu's NameIsBuiltinVar, which goes on declaring it, and a match pattern keeps
/// the pattern parser's errors: lowered, reported as nu reports them.
#[rstest]
#[case::binding("let nu = 1", &[("NameIsBuiltinVar", "nu")])]
#[case::parameter("def f [in] {}", &[("NameIsBuiltinVar", "in")])]
#[case::closure_parameter("do {|$nu| }", &[("NameIsBuiltinVar", "$nu")])]
#[case::pattern_variable("match 1 { $in => 1 }", &[("NameIsBuiltinVar", "$in")])]
#[case::loop_variable("for nu in [1] {}", &[("NameIsBuiltinVar", "nu")])]
#[case::pattern_literal("match 1 { 0b2 => 1 }", &[("InvalidLiteral", "0b2")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn reserved_variable_name_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A record where a keyword takes a block or a handler's closure is nu's type mismatch, with
/// garbage in the argument's place (the `catch` keyword and its value for a handler): lowered,
/// reported as there.
#[rstest]
#[case::while_body("while true {a: 1}", &[("TypeMismatch", "{a: 1}")])]
#[case::catch_handler("try {} catch {a: 1}", &[("TypeMismatch", "catch {a: 1}")])]
#[case::if_branch("if true {a: 1}", &[("TypeMismatch", "{a: 1}")])]
#[case::nested_record("loop {a: {b: 1}}", &[("TypeMismatch", "{a: {b: 1}}")])]
#[case::try_body("try {a: 1}", &[("TypeMismatch", "{a: 1}")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn record_in_a_block_place_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A record or list with an error of its own (an unexpected token, a missing `:`, a field named
/// twice, a `;` in a list) is nu's error, which its record or list parser goes on past:
/// lowered, the item read again by that parser.
#[rstest]
#[case::unexpected_key("{=: 1}", &[("LabeledErrorWithHelp", "=")])]
#[case::missing_colon("{a: 1 b}", &[("LabeledErrorWithHelp", "")])]
#[case::list_semicolon("[1; 2]", &[("LabeledErrorWithHelp", ";")])]
#[case::field_twice("{a: 1, a: 2}", &[("ColumnDefinedTwice", "a")])]
#[case::call_argument("echo {a: =}", &[("LabeledErrorWithHelp", "=")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn malformed_record_or_list_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A definition name nu refuses (whitespace other than single spaces, a size, a parser keyword)
/// is reported by its predeclaration and again by `finish_def`/`finish_extern`, which find no
/// predeclared command: lowered, and reported as there.
#[rstest]
#[case::extern_trailing_space(
    r#"extern "npm root " []"#,
    &[("CommandDefNotValid", r#""npm root ""#), ("InternalError", r#""npm root ""#)]
)]
#[case::parser_keyword(
    "def if [] {}",
    &[("NameIsKeyword", "if"), ("NameIsKeyword", "if")]
)]
#[case::double_space(
    r#"def "a  b" [] { 1 }"#,
    &[("CommandDefNotValid", r#""a  b""#), ("InternalError", r#""a  b""#)]
)]
#[case::byte_size(
    "def 1kb [] {}",
    &[("CommandDefNotValid", "1kb"), ("InternalError", "1kb")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn refused_definition_name_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A signature nu's `parse_full_signature` reports an error in (a name taken twice, a bad
/// parameter name, a default of the wrong type, a required parameter after an optional one) is
/// the argument of nu's call to `def` or `extern`, which goes on to define the command: lowered,
/// the signature read by that parser.
#[rstest]
#[case::duplicate_parameter("def f [a, a] { }", &[("DuplicateParameter", "a")])]
#[case::parameter_name("def f [a.b] {}", &[("Expected", "a.b")])]
#[case::extern_flag_name("extern e [--a.b]", &[("Expected", "--a.b")])]
#[case::default_type(
    "def f [x: int = abc] {}",
    &[("Expected", "abc"), ("NonConstantDefaultValue", "abc")]
)]
#[case::required_after_optional("def f [a?, b] {}", &[("RequiredAfterOptional", "a?, b")])]
#[case::signature_of_two_items("def f [a, a] {} {}", &[("DuplicateParameter", "a")])]
#[case::exported(
    "export def f [-x, -x] {}",
    &[("DuplicateParameter", "-x"), ("DuplicateParameter", "-x")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn signature_error_is_lowered(#[case] code: &str, #[case] expected: &[(&str, &str)]) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A statement where nu reads a command (in a pipeline of several commands, after an
/// environment shorthand) is nu's `BuiltinCommandInPipeline`, at the element's first item, and
/// is read as a call: lowered, reported as there.
#[rstest]
#[case::first_of_a_pipeline("use std | length", &[("BuiltinCommandInPipeline", "use")])]
#[case::after_a_shorthand("FOO=1 use std", &[("BuiltinCommandInPipeline", "FOO=1")])]
#[case::two_word_head(
    "ls | export def f [] {}",
    &[("BuiltinCommandInPipeline", "export"), ("InputMismatch", "export def")]
)]
#[case::overlay(
    "ls | overlay use spam",
    &[("BuiltinCommandInPipeline", "overlay"), ("InputMismatch", "overlay use")]
)]
#[case::plugin_use("plugin use x | ls", &[("BuiltinCommandInPipeline", "plugin")])]
#[case::overlay_list_after_a_shorthand(
    "FOO=1 overlay list",
    &[("BuiltinCommandInPipeline", "FOO=1")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn statement_in_a_pipeline_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A statement in a pipeline whose words name an alias of an external command is no call to
/// a command but an external call: nu still reports the statement, so the winnow parser
/// leaves it to the classic parser.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn statement_through_an_external_alias_in_a_pipeline() -> Result {
    let code = "alias \"source me\" = ^echo hi\n[1] | source me";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        matches!(errors.as_slice(), [error] if error.starts_with("BuiltinCommandInPipeline")),
        "`{code}`: {errors:?}"
    );
    Ok(())
}

/// Attributes with no definition after them (none, a command that is no definition, a blank
/// line) are nu's `AttributeRequiresDefinition` at the last one, each attribute read first:
/// lowered, reported as there.
#[rstest]
#[case::alone("@example \"x\" { 1 }", &[("AttributeRequiresDefinition", "example \"x\" { 1 }")])]
#[case::before_a_command("@deprecated\nls", &[("AttributeRequiresDefinition", "deprecated")])]
#[case::blank_line_before_a_definition(
    "@example \"x\" { 1 }\n\ndef f [] {}",
    &[("AttributeRequiresDefinition", "example \"x\" { 1 }")]
)]
#[case::attribute_error(
    "@example x",
    &[("MissingPositional", ""), ("AttributeRequiresDefinition", "example x")]
)]
#[case::unknown_attribute(
    "@x",
    &[("UnknownCommand", "x"), ("AttributeRequiresDefinition", "x")]
)]
#[case::two_then_semicolon(
    "@deprecated\n@search-terms a b;",
    &[("AttributeRequiresDefinition", "search-terms a b")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attributes_without_a_definition_are_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// Attributes before an `export` that exports no definition are read by nu's export
/// (`parse_export_in_block`), not `parse_attribute_block`: its errors are the classic parser's.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attributes_before_an_export_of_no_definition() -> Result {
    let code = "@example \"x\" { 1 }\nexport alias y = ls";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        matches!(
            errors.as_slice(),
            [first, second] if first.starts_with("AttributeRequiresDefinition")
                && second.starts_with("InternalError")
        ),
        "`{code}`: {errors:?}"
    );
    Ok(())
}

/// `%name` naming no built-in command is nu's error at the name, after which nu reads the items
/// after the `%` as an external call: lowered, reported as there.
#[rstest]
#[case::bare_name("%foo", &[("LabeledErrorWithHelp", "foo")])]
#[case::several_words("%my cmd", &[("LabeledErrorWithHelp", "my")])]
#[case::quoted_name("%\"ls\" a", &[("LabeledErrorWithHelp", "\"ls\"")])]
#[case::in_a_pipeline("ls | %foo x", &[("LabeledErrorWithHelp", "foo")])]
#[case::caret_after("%^foo", &[("LabeledErrorWithHelp", "^foo")])]
#[case::second_sigil("%%ls", &[("LabeledErrorWithHelp", "%ls")])]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn percent_call_of_no_builtin_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// After a lone `%` nu's external call has an empty head (the item after it is an argument),
/// which the winnow parser leaves to the classic parser; otherwise the head is the item after
/// the `%`, read as an external command's name.
#[rstest]
#[case::lone_sigil("% foo", "")]
#[case::caret_after("%^foo", "^foo")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn percent_call_head_as_nu_reads_it(#[case] code: &str, #[case] head: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let Expr::ExternalCall(call_head, _) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: the statement is not an external call");
    };
    assert_eq!(
        working_set.get_span_contents(call_head.span),
        head.as_bytes(),
        "`{code}`"
    );
    Ok(())
}

/// A statement alone in its pipeline that nu refuses to redirect is its
/// `RedirectingBuiltinCommand`, named as its parse function names it, at the redirection's
/// operators, and garbage over its items (a `for` over its keyword): lowered so.
#[rstest]
#[case::definition("def f [] { } o> x", "def", "o>", None, "def f [] { }")]
#[case::for_loop("for x in [1] { } o> x", "for", "o>", None, "for")]
#[case::separate_streams("def f [] { } e> y o> x", "def", "e>", Some("o>"), "def f [] { }")]
#[case::export_def(
    "export def f [] {} o> x",
    "export def",
    "o>",
    None,
    "export def f [] {}"
)]
#[case::keyword_call("hide ls o> x", "hide", "o>", None, "hide ls")]
#[case::overlay("overlay list e> x", "overlay", "e>", None, "overlay list")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn redirected_statement_is_lowered(
    #[case] code: &str,
    #[case] name: &str,
    #[case] first: &str,
    #[case] second: Option<&str>,
    #[case] garbage: &str,
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`"
    );
    let [ParseError::RedirectingBuiltinCommand(found, found_first, found_second)] =
        working_set.parse_errors.as_slice()
    else {
        panic!("`{code}`: {:?}", working_set.parse_errors);
    };
    assert_eq!(*found, name, "`{code}`");
    assert_eq!(
        working_set.get_span_contents(*found_first),
        first.as_bytes()
    );
    assert_eq!(
        found_second.map(|span| working_set.get_span_contents(span).to_vec()),
        second.map(|text| text.as_bytes().to_vec()),
        "`{code}`"
    );
    let expression = &block.pipelines[0].elements[0].expr;
    assert!(matches!(expression.expr, Expr::Garbage), "`{code}`");
    assert_eq!(
        working_set.get_span_contents(expression.span),
        garbage.as_bytes(),
        "`{code}`"
    );
    Ok(())
}

/// In a pipeline of several commands nu reads each where a command is (`parse_expression`),
/// which refuses no redirection: lowered, without an error.
#[rstest]
#[case::source_env("source-env null o+e>| ls")]
#[case::overlay_list("overlay list e>| ls")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn keyword_command_redirected_into_a_pipeline(#[case] code: &str) -> Result {
    assert_eq!(super::winnow_calls::lowered_errors(code), [], "`{code}`");
    Ok(())
}

/// An alias of a parser keyword nu cannot alias is its `CantAliasKeyword` at the target's head,
/// and no alias: lowered, reported as there.
#[rstest]
#[case::definition("alias d = def", "def")]
#[case::loop_keyword("alias x = for", "for")]
#[case::two_word_head("alias e = export def", "export def")]
#[case::binding("alias l = let", "let")]
#[case::row_condition("alias w = where", "where")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_a_parser_keyword_is_lowered(#[case] code: &str, #[case] head: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("CantAliasKeyword".to_string(), head.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// An alias of a parser keyword with arguments is left to the classic parser, whose first
/// reading of the target reads some keywords their own way (a `def`'s body is a closure
/// whatever its shape): nu reports only the alias.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_a_parser_keyword_with_arguments() -> Result {
    let code = "alias zz = def foo [] {a: 1}";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let errors: Vec<String> = working_set
        .parse_errors
        .iter()
        .map(|error| format!("{error:?}"))
        .collect();
    assert!(
        matches!(errors.as_slice(), [error] if error.starts_with("CantAliasKeyword")),
        "`{code}`: {errors:?}"
    );
    Ok(())
}

/// A math expression with an item where an operator goes that is none is nu's error at that
/// item, after which nu's math parser goes on: lowered, reported as there, in a statement, a
/// row condition, a condition and a subexpression.
#[rstest]
#[case::unknown_operator(
    "1 pow 2",
    &[("UnknownOperator", "pow"), ("IncompleteMathExpression", "pow")]
)]
#[case::no_operator(
    "1 2",
    &[("Expected", "2"), ("IncompleteMathExpression", "2"), ("IncompleteMathExpression", "2")]
)]
#[case::row_condition(
    "[1 2] | where $it pow 2",
    &[("UnknownOperator", "pow"), ("IncompleteMathExpression", "pow")]
)]
#[case::condition(
    "if 1 pow 2 { 3 }",
    &[("UnknownOperator", "pow"), ("IncompleteMathExpression", "pow")]
)]
#[case::subexpression(
    "print (1 pow 2)",
    &[("UnknownOperator", "pow"), ("IncompleteMathExpression", "pow")]
)]
#[case::quoted_first_item(
    "\"a b\" is \"c\"",
    &[("UnknownOperator", "is"), ("IncompleteMathExpression", "is")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn math_with_a_bad_operator_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// An alias's words take `|` for an item, which the text of the condition read again would
/// not: the bad operator is the `|`, as nu reports it.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn bad_operator_in_an_alias_target() -> Result {
    let code = "alias zz = if [1 2] | is-empty { }\nzz";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let errors: Vec<(String, String)> = working_set
        .parse_errors
        .iter()
        .map(|error| {
            let debug = format!("{error:?}");
            let kind = debug.split(['(', ' ', '{']).next().unwrap_or_default();
            let text = working_set.get_span_contents(error.span());
            (kind.to_owned(), String::from_utf8_lossy(text).into_owned())
        })
        .collect();
    assert_eq!(
        errors,
        [
            ("Expected".to_string(), "|".to_string()),
            ("IncompleteMathExpression".to_string(), "|".to_string())
        ],
        "`{code}`"
    );
    Ok(())
}

/// In a row condition nu's math parser reads a bare word before an operator as a cell path of
/// the row, a bad operator after it too.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn row_condition_with_a_bad_operator_reads_the_row() -> Result {
    let code = "[[name]; [1]] | where name pow 2";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let Expr::Call(call) = &block.pipelines[0].elements[1].expr.expr else {
        panic!("`{code}`: the second element is not a call");
    };
    let Some(Expr::RowCondition(block_id)) = call.positional_iter().next().map(|value| &value.expr)
    else {
        panic!("`{code}`: no row condition");
    };
    let condition = working_set.get_block(*block_id);
    let Expr::BinaryOp(lhs, _, _) = &condition.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: the condition is not an operation");
    };
    assert!(
        matches!(lhs.expr, Expr::FullCellPath(_)),
        "`{code}`: {:?}",
        lhs.expr
    );
    Ok(())
}

/// Closure parameters where a keyword wants a block are nu's Mismatch, with garbage in the
/// block's place: lowered, reported as there.
#[rstest]
#[case::if_block("if true { |x| $x }", "{ |x| $x }")]
#[case::while_block("while false { |y| $y }", "{ |y| $y }")]
#[case::for_block("for x in [1] { |y| $y }", "{ |y| $y }")]
#[case::loop_block("loop { |x| break }", "{ |x| break }")]
#[case::try_block("try { |x| 1 }", "{ |x| 1 }")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn closure_params_in_a_block_are_lowered(#[case] code: &str, #[case] braces: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("Mismatch".to_string(), braces.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// A cell path with a bad tail (a member followed by what may not follow it, a member with `(`
/// in it) is nu's error, after which nu's cell-path parser goes on: lowered, reported as there.
#[rstest]
#[case::second_question_mark("$env.a??", "?")]
#[case::equals_after("$env.a?=", "=")]
#[case::parenthesis_in_a_member("$env.a(b)", "a(b)")]
#[case::after_a_subexpression("(0)..\"a\"", "\"a\"")]
#[case::record_value("{a: $env.a!!}", "!")]
#[case::closure_body("[1] | each {|r| $r.a?? }", "?")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn cell_path_with_a_bad_tail_is_lowered(#[case] code: &str, #[case] at: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("Expected".to_string(), at.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// A cell path with a bad tail where nothing goes on past it (an operand of a row condition)
/// fails its statement, which the classic parser parses: nu's error alone.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn bad_cell_path_tail_in_a_row_condition() -> Result {
    let code = "[1] | where 'a'..'b'";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let errors: Vec<(String, String)> = working_set
        .parse_errors
        .iter()
        .map(|error| {
            let debug = format!("{error:?}");
            let kind = debug.split(['(', ' ', '{']).next().unwrap_or_default();
            let text = working_set.get_span_contents(error.span());
            (kind.to_owned(), String::from_utf8_lossy(text).into_owned())
        })
        .collect();
    assert_eq!(
        errors,
        [("Expected".to_string(), "'b'".to_string())],
        "`{code}`"
    );
    Ok(())
}

/// A match pattern with an error is nu's error, after which nu's pattern parser gives a pattern
/// still: lowered, reported as there.
#[rstest]
#[case::cell_path_variable("match 1 { $env.PWD => k }", "Expected", "$env.PWD")]
#[case::dash_in_a_variable("match 1 { $a-b => 1 }", "Expected", "$a-b")]
#[case::record_without_colon("match {a: 1} { {a 1} => 1 }", "Expected", "{a 1}")]
#[case::two_rests(
    "match [] { [.., 2, ..] => true, _ => false }",
    "LabeledErrorWithHelp",
    ".."
)]
#[case::in_an_alternative("match 1 { $a-b | 2 => k }", "Expected", "$a-b")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn match_pattern_with_an_error_is_lowered(
    #[case] code: &str,
    #[case] kind: &str,
    #[case] at: &str,
) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [(kind.to_string(), at.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// A definition name with its parameters joined to it is nu's error (at the byte before the
/// parameters), with garbage over the statement, and an export of it is no call: lowered,
/// reported as there.
#[rstest]
#[case::definition("def cmd[] {}", &[("LabeledErrorWithHelp", "")])]
#[case::extern_definition("extern cmd(--flag)", &[("LabeledErrorWithHelp", "")])]
#[case::bracketed_name("def [foo] [] {}", &[("LabeledErrorWithHelp", "")])]
#[case::exported(
    "export def cmd[x] {}",
    &[("LabeledErrorWithHelp", ""), ("InternalError", "def cmd[x] {}")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn parameters_joined_to_a_name_are_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// nu refuses a name with its parameters joined to it in the definition's scope, which it
/// enters and leaves open.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn parameters_joined_to_a_name_leave_a_scope_open() -> Result {
    let engine_state = test().engine_state;
    let depth = |code: &str| {
        let mut working_set = StateWorkingSet::new(&engine_state);
        nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
        working_set.delta.scope.len()
    };
    assert_eq!(depth("def cmd[] {}"), depth("ls") + 1);
    Ok(())
}

/// A table with an error of its own (a row that is no list, no row, a spread in a row, a row
/// short of columns) is nu's error, which its table parser goes on past: lowered, the table read
/// again by that parser.
#[rstest]
#[case::row_not_a_list("[[a]; 1]", "LabeledErrorWithHelp", "1")]
#[case::no_row("[[a b];]", "Expected", "")]
#[case::spread_in_a_row("[[a]; [...[1]]]", "LabeledError", "[1]")]
#[case::missing_columns("[[a b]; [1]]", "MissingColumns", "[1]")]
#[case::call_argument("echo [[a]; 1]", "LabeledErrorWithHelp", "1")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn malformed_table_is_lowered(#[case] code: &str, #[case] kind: &str, #[case] at: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [(kind.to_string(), at.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// An assignment whose value starts with a bare external command is nu's error at the command,
/// and the assignment stays: lowered, reported as there.
#[rstest]
#[case::variable("mut x = 1; $x = git", "git")]
#[case::environment("$env.FOO = bar", "bar")]
#[case::pipeline("mut x = 1; $x = foo bar | length", "foo")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn bare_external_in_an_assignment_is_lowered(#[case] code: &str, #[case] head: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("LabeledErrorWithHelp".to_string(), head.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// An alias of an expression is nu's `CantAliasExpression`, naming what the target reads as, at
/// its first item, and no alias: lowered, reported as there.
#[rstest]
#[case::integer("alias a = 1", "an integer", "1")]
#[case::subexpression("alias a = (ls)", "a cell path expression", "(ls)")]
#[case::operation("alias a = 1 + 2", "a binary operation", "1")]
#[case::flag_like("alias a = --long", "a string", "--long")]
#[case::garbage("alias a = not", "a garbage expression", "not")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_an_expression_is_lowered(
    #[case] code: &str,
    #[case] description: &str,
    #[case] at: &str,
) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("CantAliasExpression".to_string(), at.to_string())],
        "`{code}`"
    );
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let [ParseError::CantAliasExpression(found, _)] = working_set.parse_errors.as_slice() else {
        panic!("`{code}`: {:?}", working_set.parse_errors);
    };
    assert_eq!(found, description, "`{code}`");
    Ok(())
}

/// The `=` argument of an alias of an expression is what nu's `parse_expression` reads: a lone
/// `not` is garbage.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_an_expression_keeps_what_nu_reads() -> Result {
    let code = "alias a = not";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let Expr::Call(alias) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: the statement is not a call");
    };
    let Some(Expr::Keyword(keyword)) = alias.positional_iter().nth(1).map(|value| &value.expr)
    else {
        panic!("`{code}`: the alias has no `=` argument");
    };
    assert!(matches!(keyword.expr.expr, Expr::Garbage), "`{code}`");
    Ok(())
}

/// A target the winnow parser reads as an expression but nu does not (`0..$x.len`) is an
/// alias: nu declares it.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_what_nu_reads_as_a_call() -> Result {
    let code = "alias zz = 0..$x.len";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, [], "`{code}`");
    assert!(working_set.find_decl(b"zz").is_some(), "`{code}`: no alias");
    Ok(())
}

/// An alias with a misplaced item (no name, no `=` second) is nu's AssignmentMismatch before
/// anything else, with garbage over the item: lowered, reported as there.
#[rstest]
#[case::missing_name("alias = ls", "=")]
#[case::dashes_for_a_name("alias -- x = ls", "x")]
#[case::help_in_place_of_the_sign("alias x --help extra", "--help")]
#[case::help_before_the_name("alias --help x = ls", "x")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn misplaced_alias_item_is_lowered(#[case] code: &str, #[case] at: &str) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [("AssignmentMismatch".to_string(), at.to_string())],
        "`{code}`"
    );
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let expression = &block.pipelines[0].elements[0].expr;
    assert!(matches!(expression.expr, Expr::Garbage), "`{code}`");
    assert_eq!(
        working_set.get_span_contents(expression.span),
        at.as_bytes(),
        "`{code}`"
    );
    Ok(())
}

/// An alias name nu refuses (no string, an unsupported name, a parser keyword) is its error,
/// with the call to `alias` garbage and no alias declared: lowered, reported as there.
#[rstest]
#[case::variable("alias $x = a", "AliasNotValid", "$x")]
#[case::hash_in_the_name("alias \"x#y\" = ls", "AliasNotValid", "\"x#y\"")]
#[case::parser_keyword("alias if = ls", "NameIsKeyword", "if")]
#[case::subexpression("alias (x) = ls", "AliasNotValid", "(x)")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn refused_alias_name_is_lowered(
    #[case] code: &str,
    #[case] kind: &str,
    #[case] at: &str,
) -> Result {
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        [(kind.to_string(), at.to_string())],
        "`{code}`"
    );
    Ok(())
}

/// No alias is declared for a name nu refuses: a later `x#y` is an external command.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn refused_alias_name_declares_nothing() -> Result {
    let code = "alias \"x#y\" = ls\nx#y";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert!(
        matches!(
            block.pipelines[1].elements[0].expr.expr,
            Expr::ExternalCall(..)
        ),
        "`{code}`"
    );
    Ok(())
}

/// A call for help in a module body, which the keyword's parse function gives back as the
/// statement, is one of the body's statements (`parse_module_block` dispatches by the first
/// word), lowered as at top level.
#[rstest]
#[case::def("module m { def --help }")]
#[case::def_after_body("module m { def f [] {} --help }")]
#[case::extern_command("module m { extern --help }")]
#[case::alias("module m { alias --help }")]
#[case::type_alias("module m { type --help }")]
#[case::use_module("module m { use --help }")]
#[case::submodule("module m { module --help }")]
#[case::constant("module m { const x --help }")]
#[case::export_env("module m { export-env --help }")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn call_for_help_in_a_module_body_is_lowered(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    Ok(())
}

/// A `module` statement that is more than a name, a block and a `--` after it is the call
/// `parse_module` makes first (`ArgumentParsingLevel::FirstK`: the name read by its shape, any
/// later positional garbage), whose errors are a call's: lowered, reported as there. A valid
/// call goes on with the item after the name as the body, whatever it is.
#[rstest]
#[case::flag_after_body(
    "module x { export def f [] { 1 } } --foo",
    &[("UnknownFlag", "--foo")]
)]
#[case::flag_before_body(
    "module x --foo { export def f [] { 1 } }",
    &[("UnknownFlag", "--foo"), ("Expected", "--foo")]
)]
#[case::missing_name("module", &[("MissingPositional", "")])]
#[case::extra_positional("module x {} extra", &[("ExtraPositional", "extra")])]
#[case::help_body_not_read("module x --help {ls}", &[])]
#[case::spread_is_no_name(
    "let l = ['x']; module ...$l {}",
    &[("UnexpectedSpreadArg", "...$l"), ("TypeMismatch", "{}"), ("UnknownState", "module ...$l {}")]
)]
#[case::end_of_options_before_body("module x -- {}", &[("Expected", "--")])]
#[case::record_name("module {}", &[("TypeMismatch", "{}"), ("UnknownState", "module {}")])]
#[case::export_without_block(
    "export module x y",
    &[("Expected", "y"), ("InternalError", "module x y")]
)]
#[case::export_in_a_module(
    "module p { export module x --foo {} }",
    &[("UnknownFlag", "--foo"), ("Expected", "--foo"), ("InternalError", "module x --foo {}")]
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_read_as_a_call_is_lowered(
    #[case] code: &str,
    #[case] expected: &[(&str, &str)],
) -> Result {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, text)| (kind.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        super::winnow_calls::lowered_errors(code),
        expected,
        "`{code}`"
    );
    Ok(())
}

/// A valid `module` call adds the module only when the item after its name is a block: a flag
/// after the body leaves it there, a flag before the body takes its place.
#[rstest]
#[case::flag_after_body("module x {} --foo", true)]
#[case::flag_before_body("module x --foo {}", false)]
#[case::extra_positional("module x {} extra", true)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn module_call_adds_a_module_with_a_block_after_the_name(
    #[case] code: &str,
    #[case] added: bool,
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    assert_eq!(working_set.find_module(b"x").is_some(), added, "`{code}`");
    Ok(())
}

/// A row condition or a math expression of several items, which the winnow parser read one by
/// one, is read again by it as one value, with the commands the working set knows: a condition
/// after a flag, which may call a command of the file, and the conditions of an alias's `if`.
#[rstest]
#[case::condition_after_a_flag("[1 2 3 4] | take while --include 1 $it < 3 | length", 3)]
#[case::condition_calls_a_command(
    "def two [] { 2 }\n[1 2 3 4] | take while --include 1 $it < (two) | length",
    2
)]
#[case::bare_word_condition("[{a: 1} {a: 5} {a: 2}] | take while --include 0 a < 3 | length", 1)]
#[case::alias_target("let x = 1\nalias z = if $x > 0 { 1 } else { 2 }\nz", 1)]
#[case::alias_target_else_if(
    "let x = 1\nalias z = if $x > 1 { 1 } else if $x > 0 { 2 } else { 3 }\nz",
    2
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn several_items_read_as_one_value_are_lowered(
    #[case] code: &str,
    #[case] expected: i64,
) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    test().run(code).expect_value_eq(expected)
}

/// Items the condition takes that are no math expression stay the classic parser's error:
/// without an `else`, `if true {} --help` gives the condition `true {}`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn condition_of_items_that_is_no_expression_is_an_error() -> Result {
    let code = "if true {} --help";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert!(!working_set.parse_errors.is_empty(), "`{code}`");
    Ok(())
}

/// A parameter's defaults are read in turn, as `parse_signature_helper` reads them, each with
/// the shape the parameter has then: without a type, `a` makes it a string and `1` is then the
/// string `"1"`; the last default wins.
#[rstest]
#[case::untyped_positional("def f [x = a = 1] { $x | describe }\nf", "string")]
#[case::untyped_flag("def f [--n = 1 = 2] { $n | describe }\nf", "int")]
#[case::three_defaults("def f [x = a = b = 1] { $x }\nf", "1")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn every_default_of_a_parameter_is_read(#[case] code: &str, #[case] expected: &str) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    test().run(code).expect_value_eq(expected)
}

/// Every default of a parameter must be a constant, the ones a later default replaces too.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn earlier_default_that_is_no_constant_is_an_error() -> Result {
    let code = "def f [x: any = $env.PWD = 1] {}";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert!(
        matches!(
            working_set.parse_errors.as_slice(),
            [nu_protocol::ParseError::NonConstantDefaultValue(_)]
        ),
        "`{code}`: {:?}",
        working_set.parse_errors
    );
    Ok(())
}

/// A `def` body with parameters is a closure as `parse_closure_expression` reads it, its
/// parameters declared in its own scope; `finish_def` then gives it the command's signature.
#[rstest]
#[case::parameter_read("def f [] {|x| $x }")]
#[case::exported("export def f [] {|x| }")]
#[case::called("def f [a] {|x| $a }\nf 3")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn def_body_with_parameters_is_lowered(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let (working_set, _) = parse_lowered(&engine_state, code);
    let decl_id = working_set.find_decl(b"f").expect("f is defined");
    let block_id = working_set
        .get_decl(decl_id)
        .block_id()
        .expect("f has a body");
    let signature = &working_set.get_block(block_id).signature;
    assert_eq!(
        signature.name, "f",
        "`{code}`: the body keeps the command's signature"
    );
    Ok(())
}

/// `NAME=value` is an environment shorthand only when `parse_string_strict` reads its value:
/// one with a space outside quotes (`FOO=(1 + 1)`, `FOO=[1 2]`) makes the item the command, an
/// external one; a quoted value with a space stays a shorthand, a call to `with-env`.
#[rstest]
#[case::subexpression("FOO=(1 + 1) ls", false)]
#[case::list("FOO=[1 2] ls", false)]
#[case::backtick("FOO=`a b` ls", false)]
#[case::quoted("FOO='a b' ls", true)]
#[case::no_space("FOO=(1+1) ls", true)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn shorthand_value_with_a_space_makes_a_command(
    #[case] code: &str,
    #[case] shorthand: bool,
) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let expression = &block.pipelines[0].elements[0].expr.expr;
    match expression {
        Expr::Call(call) if shorthand => {
            assert_eq!(
                working_set.get_decl(call.decl_id).name(),
                "with-env",
                "`{code}`"
            )
        }
        Expr::ExternalCall(..) if !shorthand => {}
        other => panic!("`{code}`: {other:?}"),
    }
    Ok(())
}

/// An alias's target starting with an environment shorthand is, as `alias`'s argument, a call
/// to `with-env` (`parse_expression`, where a `|` is a word too), and the alias's body the
/// external command `FOO=1` (`parse_call`).
#[rstest]
#[case::command("alias x = FOO=1 ls")]
#[case::pipe_is_a_word("alias x = FOO=1 ls | length")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn alias_of_an_environment_shorthand_is_lowered(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: not a call");
    };
    let Some(Expr::Keyword(keyword)) = call.positional_iter().nth(1).map(|e| &e.expr) else {
        panic!("`{code}`: no `=` argument: {call:?}");
    };
    let Expr::Call(argument) = &keyword.expr.expr else {
        panic!("`{code}`: the argument is no call: {keyword:?}");
    };
    assert_eq!(working_set.get_decl(argument.decl_id).name(), "with-env");
    let alias = working_set.find_decl(b"x").expect("the alias is defined");
    let wrapped = &working_set
        .get_decl(alias)
        .as_alias()
        .expect("an alias")
        .wrapped_call;
    assert!(
        matches!(wrapped.expr, Expr::ExternalCall(..)),
        "`{code}`: {wrapped:?}"
    );
    Ok(())
}

/// nu picks `run` by its first word wherever it reads a command, and `source`, `source-env` and
/// `hide` at the start of a statement, whatever longer command name starts with it; after a
/// `|` or a shorthand, `source-env` is the longer command (`parse_call`).
#[rstest]
#[case::hide("def me [] { 1 }\ndef \"hide me\" [] { 'custom' }\nhide me", "hide")]
#[case::source("def \"source null\" [] { 'custom' }\nsource null", "source")]
#[case::run_in_a_pipeline("def \"run null\" [] { 'custom' }\n[1] | run null", "run")]
#[case::source_env_in_a_pipeline(
    "def \"source-env me\" [] { 'custom' }\n[1] | source-env me",
    "source-env me"
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn first_word_command_is_lowered(#[case] code: &str, #[case] command: &str) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let last = &block.pipelines.last().expect("a statement").elements;
    let Expr::Call(call) = &last.last().expect("an element").expr.expr else {
        panic!("`{code}`: not a call");
    };
    assert_eq!(
        working_set.get_decl(call.decl_id).name(),
        command,
        "`{code}`"
    );
    Ok(())
}

/// A call to a command `hide` hid is what `parse_call` makes of it now: an external call, or a
/// call to a shorter command name that is still there, the rest of the name its first argument.
#[test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn call_to_a_hidden_command_is_lowered() -> Result {
    let code = "def f [] { 1 }\nhide f\nf a";
    let engine_state = test().engine_state;
    let (_, block) = parse_lowered(&engine_state, code);
    let last = &block.pipelines.last().expect("a statement").elements[0].expr;
    assert!(
        matches!(last.expr, Expr::ExternalCall(..)),
        "`{code}`: {last:?}"
    );

    let code = "def f [x] { $x }\ndef \"f g\" [] { 2 }\nhide \"f g\"\nf g";
    parse_lowered(&engine_state, code);
    test().run(code).expect_value_eq("g")
}

/// `let` with a type and no value declares the variable with that type
/// (`parse_var_with_opt_type`), a type of several items too.
#[rstest]
#[case::int("let x: int", Type::Int)]
#[case::record("let x: record<a: int>", Type::Record([("a", Type::Int)].into()))]
#[case::piped("'s' | let x: string", Type::String)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn typed_let_without_a_value_is_lowered(#[case] code: &str, #[case] ty: Type) -> Result {
    let engine_state = test().engine_state;
    let (working_set, block) = parse_lowered(&engine_state, code);
    let last = &block.pipelines[0].elements.last().expect("an element").expr;
    let Expr::Call(call) = &last.expr else {
        panic!("`{code}`: not a call");
    };
    let variable = call.positional_iter().next().expect("a variable");
    let var_id = variable.as_var().expect("a variable declaration");
    assert_eq!(working_set.get_variable(var_id).ty, ty, "`{code}`");
    Ok(())
}

/// A nested block's predeclarations are read as `parse_def_predecl` reads them: a `def` or
/// `extern` call for help is predeclared from its items, and a signature naming a type the block
/// defines later reads it as `any`, the error dropped; the definition then has its own type.
#[rstest]
#[case::call_for_help("do { def f [] {} --help }", None)]
#[case::type_alias("do { type T = int; def f [x: T] { $x }; f 1 }", Some(1))]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn nested_predeclaration_is_lowered(#[case] code: &str, #[case] value: Option<i64>) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    match value {
        Some(value) => test().run(code).expect_value_eq(value),
        None => Ok(()),
    }
}

/// Arguments the winnow parser read as one row condition, for an alias of a row-condition
/// command that `hide` hid, are three external arguments for `parse_external_call`.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn hidden_row_condition_command_takes_external_arguments() -> Result {
    let code = "alias t = take while\nhide t\n[{a: 1}] | t a < 2";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let last = &block.pipelines.last().expect("a statement").elements;
    let Expr::ExternalCall(_, arguments) = &last.last().expect("an element").expr.expr else {
        panic!("`{code}`: no external call");
    };
    assert_eq!(arguments.len(), 3, "`{code}`: {arguments:?}");
    Ok(())
}

/// A call through an alias of `overlay use` in a nested block changes which commands exist for
/// the block's statements after it: the winnow parser, which tells that from the alias as it
/// parses the call (`DeclKind::OverlayAlias`), leaves the block to be parsed again statement by
/// statement, as for `overlay use` itself, instead of the classic parser taking its rest.
#[rstest]
#[case::alias_outside(
    "module m { export def f [] { 'f' } }\nalias ou = overlay use\ndo {\n  ou m\n  f\n}"
)]
#[case::alias_inside(
    "module m { export def f [] { 'f' } }\ndo {\n  alias ou = overlay use\n  ou m\n  f\n}"
)]
#[case::alias_from_a_module(
    "module m { export def f [] { 'f' } }\nmodule n { export alias ou = overlay use }\nuse n ou\ndo {\n  ou m\n  f\n}"
)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn overlay_alias_in_a_nested_block_is_lowered(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    parse_lowered(&engine_state, code);
    test().run(code).expect_value_eq("f")
}

/// A statement with a syntax error is the classic parser's alone: the winnow parser ends it
/// where the classic lite parser does, past a `|` that goes on to the next line, and lowers
/// the statements after it. Where it can't tell the end, a syntax error in the lite parse
/// itself, the classic parser takes the rest of the block: a redirection without a target,
/// which the classic lite parser finds on the next line (one pipeline), or a `|` no command
/// follows, before a blank line or before another `|`, which goes on to the next statement.
#[rstest]
#[case::pipe_starting_the_next_line("ls $x.-1\n| length\n1 + 1", 1, 2)]
#[case::pipe_ending_the_line("ls $x.-1 |\nlength\n2 + 2", 1, 2)]
#[case::semicolon("ls $x.-1; 1 + 1", 1, 2)]
#[case::end_unknown("ls o>\n1 + 1", 0, 1)]
#[case::pipe_before_a_blank_line("ls $x.-1 |\n\n1 + 1", 0, 2)]
#[case::pipe_after_a_pipe("ls $x.-1 | |\n1 + 1", 0, 1)]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn statement_after_a_syntax_error(
    #[case] code: &str,
    #[case] lowered: u64,
    #[case] pipelines: usize,
) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.error_statements - before.error_statements,
        1,
        "`{code}`"
    );
    assert_eq!(
        after.lowered_statements - before.lowered_statements,
        lowered,
        "`{code}`"
    );
    assert_eq!(block.pipelines.len(), pipelines, "`{code}`");
    Ok(())
}

/// The block of the first statement of `code`, a parenthesized subexpression, parsed with the
/// winnow front end: no statement is handed to the classic parser.
fn parenthesized_block(working_set: &mut StateWorkingSet, code: &str) -> nu_protocol::BlockId {
    let before = nu_parser::winnow_stats();
    let block = nu_parser::parse(working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.classic_statements, before.classic_statements,
        "`{code}`: a statement was handed to the classic parser"
    );
    match &block.pipelines[0].elements[0].expr.expr {
        Expr::FullCellPath(path) => match path.head.expr {
            Expr::Subexpression(block_id) => block_id,
            ref other => panic!("`{code}`: not a subexpression: {other:?}"),
        },
        other => panic!("`{code}`: not a subexpression: {other:?}"),
    }
}

/// The value of a binding is lexed again from its text as a block is (`parse_let`,
/// `parse_mut`, `parse_const`), also inside `( ... )`, whose newlines are whitespace: a newline
/// there ends the value's first pipeline, also when the value is parsed again, one statement
/// at a time, for a `use` in it.
#[rstest]
#[case::let_value("(let x = 1\n2)")]
#[case::mut_value("(mut x = 1\n2)")]
#[case::const_value("(const x = 1\n2)")]
#[case::value_parsed_again("(let x = 1\nuse std/assert)")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn binding_value_in_parentheses_is_lexed_as_a_block(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block_id = parenthesized_block(&mut working_set, code);
    let block = working_set.get_block(block_id);
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: not a call");
    };
    let value = match call.positional_iter().nth(1).map(|value| &value.expr) {
        Some(Expr::Block(block_id) | Expr::Subexpression(block_id)) => *block_id,
        other => panic!("`{code}`: no value block: {other:?}"),
    };
    assert_eq!(working_set.get_block(value).pipelines.len(), 2, "`{code}`");
    Ok(())
}

/// The head of an `export alias` call spans from `export` to `alias`, as `Span::concat` of the
/// two items, with what comes between them: in `( ... )`, a comment and a newline too.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn export_head_spans_a_comment_in_parentheses() -> Result {
    let code = "(export # c\nalias q = ls)";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block_id = parenthesized_block(&mut working_set, code);
    let block = working_set.get_block(block_id);
    let Expr::Call(call) = &block.pipelines[0].elements[0].expr.expr else {
        panic!("`{code}`: not a call");
    };
    assert_eq!(
        working_set.get_span_contents(call.head),
        b"export # c\nalias"
    );
    Ok(())
}

/// After a redirection without a target at the end of a line, the classic lite parser ends the
/// command but not its pipeline: the next line's `def` is the pipeline's next command, an error
/// there, and declares nothing, so a call to its name is an external one.
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn def_after_an_open_redirection_declares_nothing() -> Result {
    let code = "ls o>\ndef f [] {}\nf";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.find_decl(b"f"), None);
    assert!(matches!(
        block.pipelines[1].elements[0].expr.expr,
        Expr::ExternalCall(..)
    ));
    Ok(())
}

/// A definition with a syntax error goes to the classic parser with the comment after it on its
/// line, its description, as a definition that parses has it; the call after it is lowered. (An
/// alias's description is that comment's alone; a command's also comes with its
/// predeclaration.)
#[rstest]
#[case::alias("alias f = ls []]* # doc\nf")]
#[case::command("def f [] { ls $x.-1 } # doc\nf")]
#[nu_test_support::test]
#[serial]
#[exp(nu_experimental::WINNOW_PARSER)]
fn failing_definition_keeps_its_comment(#[case] code: &str) -> Result {
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let before = nu_parser::winnow_stats();
    nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    let after = nu_parser::winnow_stats();
    assert_eq!(
        after.error_statements - before.error_statements,
        1,
        "`{code}`"
    );
    assert_eq!(
        after.lowered_statements - before.lowered_statements,
        1,
        "`{code}`"
    );
    let decl = working_set.get_decl(working_set.find_decl(b"f").expect("`f` is declared"));
    assert_eq!(decl.description(), "doc", "`{code}`");
    Ok(())
}
