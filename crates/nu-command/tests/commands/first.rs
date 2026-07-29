use chrono::DateTime;
use nu_protocol::{
    Filesize, IntRange, IntoPipelineData, ListStream, PipelineData, PipelineMetadata, Range,
    Signals, Span, Value, ast::RangeInclusion,
};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[derive(Clone, Copy)]
enum InputKind {
    List,
    Range,
    Binary,
    ListStream,
}

fn range_1_to_3_exclusive() -> Value {
    let r = IntRange::new(
        Value::test_int(1),
        Value::test_int(2),
        Value::test_int(3),
        RangeInclusion::RightExclusive,
        Span::test_data(),
    )
    .expect("valid int range");
    Value::test_range(Range::IntRange(r))
}

fn pipeline_data_with_metadata(kind: InputKind, meta: Option<PipelineMetadata>) -> PipelineData {
    let span = Span::test_data();
    match kind {
        InputKind::List => Value::test_list(vec![Value::test_int(1), Value::test_int(2)])
            .into_pipeline_data_with_metadata(meta),
        InputKind::Range => range_1_to_3_exclusive().into_pipeline_data_with_metadata(meta),
        InputKind::Binary => {
            Value::binary(vec![1, 2, 3], span).into_pipeline_data_with_metadata(meta)
        }
        InputKind::ListStream => {
            let stream = ListStream::new(
                vec![Value::test_int(1), Value::test_int(2)].into_iter(),
                span,
                Signals::empty(),
            );
            PipelineData::list_stream(stream, meta)
        }
    }
}

#[test]
fn gets_first_rows_by_amount() -> Result {
    test()
        .run_with_data(
            "$in | first 3 | length",
            test_table![
                ["name", "type", "size", "modified"];
                ["los.txt", "file", Filesize::from(1), DateTime::UNIX_EPOCH.fixed_offset()],
                ["tres.txt", "file", Filesize::from(2), DateTime::UNIX_EPOCH.fixed_offset()],
                ["amigos.txt", "file", Filesize::from(3), DateTime::UNIX_EPOCH.fixed_offset()],
                ["arepas.clu", "file", Filesize::from(4), DateTime::UNIX_EPOCH.fixed_offset()],
            ],
        )
        .expect_value_eq(3)
}

#[test]
fn gets_all_rows_if_amount_higher_than_all_rows() -> Result {
    test()
        .run_with_data(
            "$in | first 99 | length",
            test_table![
                ["name", "type", "size", "modified"];
                ["los.txt", "file", Filesize::from(1), DateTime::UNIX_EPOCH.fixed_offset()],
                ["tres.txt", "file", Filesize::from(2), DateTime::UNIX_EPOCH.fixed_offset()],
                ["amigos.txt", "file", Filesize::from(3), DateTime::UNIX_EPOCH.fixed_offset()],
                ["arepas.clu", "file", Filesize::from(4), DateTime::UNIX_EPOCH.fixed_offset()],
            ],
        )
        .expect_value_eq(4)
}

#[test]
fn gets_first_row_when_no_amount_given() -> Result {
    // FIXME: We should probably change first to return a one row table instead of a record here
    test()
        .run_with_data(
            "$in | first | values | length",
            test_table![
                ["name", "type", "size", "modified"];
                ["caballeros.txt", "file", Filesize::from(1), DateTime::UNIX_EPOCH.fixed_offset()],
                ["arepas.clu", "file", Filesize::from(2), DateTime::UNIX_EPOCH.fixed_offset()],
            ],
        )
        .expect_value_eq(4)
}

#[test]
fn gets_first_row_as_list_when_amount_given() -> Result {
    test()
        .run("[1, 2, 3] | first 1 | describe")
        .expect_value_eq("list<int>")
}

#[test]
fn gets_first_bytes() -> Result {
    test()
        .run("(0x[aa bb cc] | first 2) == 0x[aa bb]")
        .expect_value_eq(true)
}

#[test]
fn gets_first_byte() -> Result {
    test().run("0x[aa bb cc] | first").expect_value_eq(170)
}

#[test]
fn gets_first_bytes_from_stream() -> Result {
    test()
        .run("(1.. | each { 0x[aa bb cc] } | bytes collect | first 2) == 0x[aa bb]")
        .expect_value_eq(true)
}

#[test]
fn gets_first_byte_from_stream() -> Result {
    test()
        .run("1.. | each { 0x[aa bb cc] } | bytes collect | first")
        .expect_value_eq(170)
}

#[test]
// covers a situation where `first` used to behave strangely on list<binary> input
fn works_with_binary_list() -> Result {
    test()
        .run("([0x[01 11]] | first) == 0x[01 11]")
        .expect_value_eq(true)
}

#[test]
fn errors_on_negative_rows() -> Result {
    let err = test().run("[1, 2, 3] | first -10").expect_shell_error()?;

    assert_matches!(err, ShellError::NeedsPositiveValue { .. });
    Ok(())
}

#[test]
fn does_not_error_on_empty_list_when_no_rows_given() -> Result {
    test()
        .run("[] | first | describe")
        .expect_value_eq("nothing")
}

#[test]
fn error_on_empty_list_when_no_rows_given_in_strict_mode() -> Result {
    let err = test()
        .run("[] | first --strict | describe")
        .expect_shell_error()?;

    assert_matches!(err, ShellError::AccessEmptyContent { .. });
    Ok(())
}

#[test]
fn gets_first_bytes_and_drops_content_type() -> Result {
    test()
        .run_with_data(
            "open $in | first 3 | metadata | get content_type? | describe",
            file!(),
        )
        .expect_value_eq("nothing")
}

#[test]
fn wrapping_first_with_optional_null_rows() -> Result {
    let code = "def wraps-first [rows?: int] { [1, 2, 3] | first $rows }; wraps-first";
    test().run(code).expect_value_eq(1)
}

#[test]
fn wrapping_first_with_optional_explicit_rows() -> Result {
    let code = "def wraps-first [rows?: int] { [1, 2, 3] | first $rows }; wraps-first 2 | length";
    test().run(code).expect_value_eq(2)
}

#[test]
fn first_bytes_with_filesize() -> Result {
    let code = "(0x[aa bb cc] | first 2b) == 0x[aa bb]";
    test().run(code).expect_value_eq(true)
}

#[test]
fn first_filesize_list_error() -> Result {
    let err = test().run("[1 2 3] | first 1kb").expect_shell_error()?;
    assert!(
        matches!(err, ShellError::IncompatibleParametersSingle { .. }),
        "expected IncompatibleParametersSingle, got {err:?}"
    );
    Ok(())
}

#[rstest]
#[case::list_first(InputKind::List, "first")]
#[case::list_first_n(InputKind::List, "first 2")]
#[case::range_first(InputKind::Range, "first")]
#[case::range_first_n(InputKind::Range, "first 2")]
#[case::list_stream_first(InputKind::ListStream, "first")]
#[case::list_stream_first_n(InputKind::ListStream, "first 2")]
#[case::binary_first(InputKind::Binary, "first")]
#[case::binary_first_n(InputKind::Binary, "first 2")]
fn first_preserves_pipeline_metadata(#[case] input: InputKind, #[case] code: &str) -> Result {
    let in_meta = Some(
        PipelineMetadata::default()
            .with_content_type(Some("text/x-test".into()))
            .with_path_columns(vec!["name".into()]),
    );
    let data = pipeline_data_with_metadata(input, in_meta.clone());
    let out = test().run_raw_with_data(code, data)?.body.take_metadata();
    let expected = if matches!(input, InputKind::Binary) {
        in_meta.clone().map(|m| m.with_content_type(None))
    } else {
        in_meta.clone()
    };
    assert_eq!(expected, out);
    Ok(())
}
