use nu_test_support::nu;
use rstest::rstest;

/// checks that garbage is highlighted as error
#[rstest]
#[case::out_pipe_as_garbage("ps out>| $in", "garbage")]
#[case::and_and_as_garbage("^foobar && ls", "garbage")]
#[case::number_redirection_as_garbage("^foobar 2> err", "garbage")]
#[case::number_redirection_as_garbage("^foobar 2>&1 err", "garbage")]
#[case::redirection_pipe_has_a_redirection_part("^ls o+e>| ls", "redirection")]
#[case::redirection_pipe_has_a_pipe_part("^ls e>| ls", "pipe")]
fn nu_highlight_color_detection(#[case] cmd: &str, #[case] shape: &str) {
    use std::fmt::Write;

    let color = "#112233";

    let mut buf = String::new();
    writeln!(&mut buf, "let color = '{color}'").unwrap();
    writeln!(
        &mut buf,
        "$env.config.color_config.shape_{} = $color",
        shape
    )
    .unwrap();
    writeln!(&mut buf, "let highlight = '{cmd}' | nu-highlight").unwrap();
    write!(&mut buf, "$highlight has (ansi $color)").unwrap();

    let outcome = nu!(buf);

    assert_eq!(outcome.out, "true");
}

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
    let actual = nu!("$env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external_resolved = '#ffffff'
        alias fff = ^rustc
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external_resolved)");

    assert_eq!(actual.out, "true");
}

#[test]
fn nu_highlight_aliased_external_unresolved() {
    let actual = nu!("$env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external = '#ffffff'
        alias fff = ^nonexist
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external)");

    assert_eq!(actual.out, "true");
}
