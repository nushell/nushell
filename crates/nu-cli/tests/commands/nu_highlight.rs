use nu_test_support::nu;

#[test]
fn nu_highlight_not_expr() {
    let actual = nu!("'not false' | nu-highlight | ansi strip");
    assert_eq!(actual.out, "not false");
}

#[test]
fn nu_highlight_where_row_condition() {
    let actual = nu!("'ls | where a b 12345(' | nu-highlight | ansi strip");
    assert_eq!(actual.out, "ls | where a b 12345(");
}
