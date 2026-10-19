use nu_protocol::{DataSource, IntoPipelineData, PipelineMetadata, Span};
use nu_test_support::prelude::*;
use rstest::rstest;

enum ExpectTo {
    Keep,
    Drop,
}

#[rstest]
#[case("42")]
#[case("-42")]
#[case("3.14")]
#[case("6.02e23")]
#[case("inf")]
#[case("NaN")]
#[case("1_000")]
#[case("1,000")]
#[case("true")]
#[case("FALSE")]
fn whitespace_padded_values_remain_strings(#[case] input: &str) -> Result {
    let mut tester = test();
    for whitespace in [" ", "\t", "\n", "\u{a0}"] {
        for padded in [
            format!("{whitespace}{input}"),
            format!("{input}{whitespace}"),
            format!("{whitespace}{input}{whitespace}"),
        ] {
            tester
                .run_with_data("detect type", padded.clone())
                .expect_value_eq(padded)?;
        }
    }
    Ok(())
}

#[rstest]
#[case("1", ExpectTo::Drop)]
#[case("01/02/2026", ExpectTo::Drop)]
#[case("true", ExpectTo::Drop)]
#[case("4hr 3min", ExpectTo::Drop)]
#[case("truee", ExpectTo::Keep)]
#[case("test", ExpectTo::Keep)]
#[case("", ExpectTo::Keep)]
#[case(" ", ExpectTo::Keep)]
#[case(" 42", ExpectTo::Keep)]
#[case(" 3.14", ExpectTo::Keep)]
#[case(" 1_000", ExpectTo::Keep)]
#[case(" true", ExpectTo::Keep)]
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
