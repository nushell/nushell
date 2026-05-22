use nu_protocol::{IntoPipelineData, PipelineMetadata, Span};
use nu_test_support::prelude::*;

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

    assert_eq!(out_meta.map(|m| m.with_content_type(None)), in_meta);
    Ok(())
}
