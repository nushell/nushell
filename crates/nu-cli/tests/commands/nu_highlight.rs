use rstest::rstest;

use nu_protocol::{IntoPipelineData, PipelineMetadata, Span};
use nu_test_support::prelude::*;

/// checks that garbage is highlighted as error
#[rstest]
#[case::out_pipe_as_garbage("ps out>| $in", "garbage")]
#[case::out_pipe_as_garbage_external("^ps out>| $in", "garbage")]
#[case::out_pipe_as_garbage_without_following_elements("ps out>|", "garbage")]
#[case::and_and_as_garbage("^foobar && ls", "garbage")]
#[case::number_redirection_as_garbage("^foobar 2> err", "garbage")]
#[case::number_merged_redirection_as_garbage("^foobar 2>&1 err", "garbage")]
#[case::redirection_pipe_has_a_redirection_part("^ls o+e>| ls", "redirection")]
#[case::redirection_pipe_has_a_pipe_part("^ls e>| ls", "pipe")]
#[case::separate_redirection_pipe_has_a_redirection_part("^ls o> foo e>| ls", "redirection")]
#[case::separate_redirection_pipe_has_a_pipe_part("^ls o> foo e>| ls", "pipe")]
#[case::separate_redirection_pipe_garbage("^ls e> foo o>| ls", "garbage")]
#[case::separate_redirection_pipe_garbage_with_a_redirection_part(
    "^ls e> foo o>| ls",
    "redirection"
)]
// https://github.com/nushell/nushell/issues/18369
#[case::leading_pipe("| ls", "garbage")]
fn nu_highlight_color_detection(#[case] cmd: &str, #[case] shape: &str) -> Result {
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

    test().run(buf).expect_value_eq(true)
}

#[test]
fn nu_highlight_not_expr() -> Result {
    test()
        .run("'not false' | nu-highlight | ansi strip")
        .expect_value_eq("not false")
}

#[test]
fn nu_highlight_where_row_condition() -> Result {
    test()
        .run("'ls | where a b 12345(' | nu-highlight | ansi strip")
        .expect_value_eq("ls | where a b 12345(")
}

#[test]
#[deps(NU)]
fn nu_highlight_aliased_external_resolved() -> Result {
    let code = "
        $env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external_resolved = '#ffffff'
        alias fff = ^nu
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external_resolved)
    ";
    test().run(code).expect_value_eq(true)
}

#[test]
fn nu_highlight_aliased_external_unresolved() -> Result {
    let code = "
        $env.config.highlight_resolved_externals = true
        $env.config.color_config.shape_external = '#ffffff'
        alias fff = ^nonexist
        ('fff' | nu-highlight) has (ansi $env.config.color_config.shape_external)
    ";
    test().run(code).expect_value_eq(true)
}

#[test]
fn nu_highlight_removes_content_type_metadata() -> Result {
    let in_meta = Some(
        PipelineMetadata::default()
            .with_content_type(Some("application/x-nuscript".into()))
            .with_data_source(nu_protocol::DataSource::FilePath("test.nu".into())),
    );
    let data = "nu-highlight"
        .into_value(Span::unknown())
        .into_pipeline_data_with_metadata(in_meta.clone());

    let out_meta = test()
        .run_raw_with_data("nu-highlight", data)?
        .take_metadata();

    assert_eq!(out_meta, in_meta.map(|m| m.with_content_type(None)));
    Ok(())
}
