use nu_protocol::{DataSource, IntoPipelineData, PipelineMetadata, Span};
use nu_test_support::prelude::*;
use rstest::rstest;

enum ExpectTo {
    Keep,
    Drop,
}

#[rstest]
#[case("1", ExpectTo::Drop)]
#[case("01/02/2026", ExpectTo::Drop)]
#[case("true", ExpectTo::Drop)]
#[case("truee", ExpectTo::Keep)]
#[case("test", ExpectTo::Keep)]
fn content_type_metadata(#[case] input: &str, #[case] expect_to: ExpectTo) -> Result {
    let in_meta = Some(
        PipelineMetadata::default()
            .with_data_source(DataSource::FilePath("test.txt".into()))
            .with_content_type(Some("text/plain".into())),
    );

    let data = input
        .into_value(Span::test_data())
        .into_pipeline_data_with_metadata(in_meta.clone());

    let out_meta = test()
        .run_raw_with_data("detect type", data)?
        .body
        .take_metadata();

    let expected = match expect_to {
        ExpectTo::Keep => in_meta,
        ExpectTo::Drop => in_meta.map(|m| m.with_content_type(None)),
    };

    assert_eq!(expected, out_meta);

    Ok(())
}
