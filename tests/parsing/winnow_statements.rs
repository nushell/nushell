//! Regression tests for the `winnow-parser` front end (statements): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{Type, Value, ast::Expr, engine::StateWorkingSet};
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
