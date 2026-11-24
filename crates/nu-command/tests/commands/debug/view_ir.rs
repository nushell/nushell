use nu_test_support::prelude::*;
use rstest::rstest;

/// `view ir` prints parser info as typed values, not as a debug dump of the parser's
/// `Expression`.
#[rstest]
#[case::import_pattern(
    "module spam { export def foo [] {} }; view ir { use spam [foo] }",
    r#""import_pattern", import "spam" ["foo"]"#
)]
// The parser adds `dirs_var` only when a `$NU_LIB_DIRS` variable is in scope.
#[case::dirs_var(
    "const NU_LIB_DIRS = []; module spam {}; view ir { use spam }",
    r#""dirs_var", var "#
)]
#[case::overlay(
    "module spam {}; view ir { overlay use spam }",
    r#""overlay_expr", module("#
)]
#[case::help_decl_id("view ir { ls --help }", r#""help_decl_id", decl "#)]
#[case::deferred_arguments(r#"let c = "ls"; view ir { %$c }"#, r#""deferred_arguments", flag"#)]
fn view_ir_shows_typed_parser_info(#[case] code: &str, #[case] expected: &str) -> Result {
    let ir: String = test().run(code)?;
    assert_contains(expected, ir.as_str());
    assert_contains_not("Expression", ir.as_str());
    Ok(())
}
