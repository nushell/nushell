use nu_protocol::{ByteStreamType, PipelineData, Span};
use nu_test_support::{fs::fixtures, prelude::*};
use pretty_assertions::assert_eq;
use rstest::rstest;

#[derive(Debug, FromValue, PartialEq)]
struct PeekRecord {
    r#type: String,
    stream: bool,
    value: Option<Vec<Value>>,
}

#[test]
fn peek_works_with_list_stream() -> Result {
    let code = "seq 1 5 | peek 2";
    let outcome = test().run_raw_with_data(code, PipelineData::Empty)?;

    let PipelineData::ListStream(stream, Some(metadata)) = outcome.body else {
        panic!("Output must be a stream with metadata")
    };

    let peek_record = PeekRecord::from_value(metadata.custom.get("peek").unwrap().clone())?;
    assert_eq!(
        peek_record,
        PeekRecord {
            r#type: "list".into(),
            stream: true,
            value: Some(
                [1, 2]
                    .into_iter()
                    .map(|x| x.into_value(Span::test_data()))
                    .collect::<Vec<_>>()
            ),
        },
    );

    stream
        .into_value()
        .map_err(Error::from)
        .expect_value_eq([1, 2, 3, 4, 5])
}

#[rstest]
#[case::binary("random binary 16b | peek 2", ByteStreamType::Binary, PeekRecord {r#type: "binary".into(), stream: true, value: None})]
#[case::string("random chars --length 16 | peek 2", ByteStreamType::String, PeekRecord {r#type: "string".into(), stream: true, value: None})]
#[case::unknown("open --raw cp/existing_file.txt | peek 2", ByteStreamType::Unknown, PeekRecord {r#type: "byte stream".into(), stream: true, value: None})]
fn peek_with_byte_streams(
    #[case] code: &str,
    #[case] byte_stream_type: ByteStreamType,
    #[case] expected_peek_record: PeekRecord,
) -> Result {
    let outcome = test()
        .cwd(fixtures())
        .run_raw_with_data(code, PipelineData::Empty)?;

    let PipelineData::ByteStream(stream, Some(metadata)) = outcome.body else {
        panic!("Output must be a binary stream with metadata")
    };

    let peek_record = PeekRecord::from_value(metadata.custom.get("peek").unwrap().clone())?;
    assert_eq!(peek_record, expected_peek_record);

    assert_eq!(stream.type_(), byte_stream_type);
    Ok(())
}
