//! Regression tests for the `winnow-parser` front end (definitions): each pins a case where it
//! parsed differently from the classic parser.

use nu_protocol::{ast::Expr, engine::StateWorkingSet};
use nu_test_support::prelude::*;
use rstest::rstest;

/// The name of a `def` or `extern` is its `string` argument, which refuses a bare `true`,
/// `false` or `null` and a `{...}`, in a script and in a module alike.
#[rstest]
#[case::def_true(r#"def true [] { "x" }"#)]
#[case::def_null(r#"def null [] { "x" }"#)]
#[case::export_def_false(r#"export def false [] { "x" }"#)]
#[case::extern_true("extern true []")]
#[case::def_braces(r#"def {a} [] { "x" }"#)]
#[case::def_in_a_module(r#"module m { export def null [] { "x" } }"#)]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn definition_name_must_read_as_a_string(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::parse_mismatch_with_full_string_msg")
}

/// A call before a `def --wrapped` resolves to its predeclaration, whose rest parameter is
/// typed when the text of the signature has `...rest:`, in a comment too (nu-parser's
/// `rest_param_is_type_annotated`): the argument `5` stays an int.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn wrapped_rest_typed_in_a_comment_before_its_definition() -> Result {
    let code = "let before = (f 5)\n\
                def --wrapped f [\n\
                ...rest # like ...rest: anything\n\
                ] { $rest | each { describe } | first }\n\
                $before";
    test().run(code).expect_value_eq("int")
}

/// A binding whose written type refuses its value is parsed again by nu-parser, which reads the
/// `$x` of the value as the `x` declared before it and reports the mismatch.
#[rstest]
#[case::let_binding("let x = 1\nlet x: string = $x\n$x")]
#[case::mut_binding("mut x = 1\nmut x: string = $x\n$x")]
#[case::const_binding("const x = 1\nconst x: string = $x\n$x")]
#[case::let_in_a_definition("def f [] { let x = 1; let x: string = $x; $x }")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn binding_type_mismatch_reads_the_variable_before(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::type_mismatch")
}

/// nu-parser reads a parameter's second default value with the type of the first. In a
/// signature `1..=5` lexes as `1..`, `=` and `5`, and `5` is no range.
#[rstest]
#[case::positional("def f [x = 1..=5] { $x | describe }; f")]
#[case::flag("def g [--n = 1..=5] { $n | describe }; g")]
#[nu_test_support::test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn second_default_value_has_the_type_of_the_first(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::parse_mismatch")
}

/// The first default of an untyped parameter gives its type to the second: in `x = a = 1`,
/// `1` is read as a string.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn second_default_value_of_an_untyped_parameter() -> Result {
    test()
        .run("def f [x = a = 1] { $x | describe }; f")
        .expect_value_eq("string")
}

/// `@@foo` calls the command `attr @foo`, the head keeping the second `@`: nu-parser's
/// `parse_attribute` drops only the first.
#[test]
#[exp(nu_experimental::WINNOW_PARSER)]
fn attribute_head_keeps_a_second_at_sign() -> Result {
    let code = "alias \"attr @foo\" = attr category\n@@foo \"x\"\ndef bar [] { 2 }";
    let engine_state = test().engine_state;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let block = nu_parser::parse(&mut working_set, None, code.as_bytes(), false);
    assert_eq!(working_set.parse_errors, []);
    let Expr::AttributeBlock(attribute_block) = &block.pipelines[1].elements[0].expr.expr else {
        panic!("`{code}`: the second statement is not an attribute block");
    };
    let Expr::Call(call) = &attribute_block.attributes[0].expr.expr else {
        panic!("`{code}`: the attribute is not a call");
    };
    assert_eq!(working_set.get_span_contents(call.head), b"@foo");
    Ok(())
}
