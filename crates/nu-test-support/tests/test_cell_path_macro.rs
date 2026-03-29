use nu_test_support_macros::test_cell_path;

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
