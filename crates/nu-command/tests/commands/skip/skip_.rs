use nu_protocol::{IntoPipelineData, PipelineMetadata, Span};
use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn skips_bytes() -> Result {
    let code = "(0x[aa bb cc] | skip 2) == 0x[cc]";
    test().run(code).expect_value_eq(true)
}

#[test]
fn skips_bytes_from_stream() -> Result {
    let code = "([0 1] | each { 0x[aa bb cc] } | bytes collect | skip 2) == 0x[cc aa bb cc]";
    test().run(code).expect_value_eq(true)
}

#[test]
fn fail_on_non_iterator() -> Result {
    let code = "1 | skip 2";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn skips_bytes_and_drops_content_type() -> Result {
    let content_type = test()
        .run_raw_with_data(
            "open $in | skip 3",
            (file!()).into_value(Span::test_data()).into_pipeline_data(),
        )?
        .take_metadata()
        .and_then(|md| md.content_type);

    assert!(content_type.is_none());
    Ok(())
}

enum InputType {
    Binary,
    List,
}

enum Metadata {
    Keep,
    Drop,
}

#[rstest]
#[case::binary_skip_0("skip 0", InputType::Binary, Metadata::Keep)]
#[case::binary_skip_1("skip 1", InputType::Binary, Metadata::Drop)]
#[case::list_skip_0("skip 0", InputType::List, Metadata::Keep)]
#[case::list_skip_1("skip 1", InputType::List, Metadata::Keep)]
fn test_with_content_type_metadata(
    #[case] code: &str,
    #[case] input_type: InputType,
    #[case] metadata: Metadata,
) -> Result {
    let in_metadata = Some(
        PipelineMetadata::default()
            .with_path_columns(vec!["name".into()])
            .with_content_type(Some("application/octet-stream".into())),
    );

    let value = match input_type {
        InputType::Binary => Value::test_binary([0x12, 0x23, 0x34, 0x45]),
        InputType::List => Value::test_list(
            [0x12, 0x23, 0x34, 0x45]
                .into_iter()
                .map(Value::test_int)
                .collect(),
        ),
    };
    let data = value.into_pipeline_data_with_metadata(in_metadata.clone());

    let out_metadata = test().run_raw_with_data(code, data)?.body.take_metadata();

    let target_metadata = match metadata {
        Metadata::Keep => in_metadata,
        Metadata::Drop => in_metadata.map(|m| m.with_content_type(None)),
    };

    assert_eq!(target_metadata, out_metadata);
    Ok(())
}
