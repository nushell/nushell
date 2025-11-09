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

#[test]
fn nu_highlight_aliased_external_resolved() {
    let actual = nu!(r#"$env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external_resolved = '#ffffff'
        alias fff = ^sleep
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external_resolved)"#);

    assert_eq!(actual.out, "true");
}

#[test]
fn nu_highlight_aliased_external_unresolved() {
    let actual = nu!(r#"$env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external = '#ffffff'
        alias fff = ^nonexist
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external)"#);

    assert_eq!(actual.out, "true");
}
