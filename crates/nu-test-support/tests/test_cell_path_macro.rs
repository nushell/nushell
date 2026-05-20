#![allow(
    unused_parens,
    reason = "the macro requires braces or parens to inline outside code"
)]

use nu_test_support::test_cell_path;

#[test]
fn builds_simple_cell_path() {
    let cell_path = test_cell_path!(foo.bar);
    assert_eq!(cell_path.to_string(), "$.foo.bar");
}

#[test]
fn builds_cell_path_with_modifiers() {
    let cell_path = test_cell_path!(foo?.bar!);
    assert_eq!(cell_path.to_string(), "$.foo?.bar!");
}

#[test]
fn builds_cell_path_with_both_modifiers() {
    let cell_path = test_cell_path!(foo?!);
    assert_eq!(cell_path.to_string(), "$.foo!?");
}

#[test]
fn builds_cell_path_with_literal_and_index() {
    let cell_path = test_cell_path!(foo."bar baz".3);
    assert_eq!(cell_path.to_string(), r#"$.foo."bar baz".3"#);
}

#[test]
fn builds_cell_path_with_inline_string_variable() {
    let column = "foo";
    let cell_path = test_cell_path!((column).bar);
    assert_eq!(cell_path.to_string(), "$.foo.bar");
}

#[test]
fn builds_cell_path_with_inline_index_variable() {
    let index: usize = 3;
    let cell_path = test_cell_path!(foo.(index));
    assert_eq!(cell_path.to_string(), "$.foo.3");
}
