use nu_protocol::{
    IntRange, IntoPipelineData, ListStream, PipelineData, PipelineMetadata, Range, Signals, Span,
    Value, ast::RangeInclusion,
};
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn gets_the_last_row() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("ls | sort-by name | where type == file | last 1 | get name.0 | path basename")
        .expect_value_eq("utf16.ini")
}

#[test]
fn gets_last_rows_by_amount() -> Result {
    Playground::setup("last_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        test()
            .cwd(dirs.test())
            .run("ls | last 3 | length")
            .expect_value_eq(3)
    })
}

#[test]
fn gets_last_row_when_no_amount_given() -> Result {
    Playground::setup("last_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        // FIXME: We should probably change last to return a one row table instead of a record here
        test()
            .cwd(dirs.test())
            .run("ls | last | values | length")
            .expect_value_eq(4)
    })
}

#[test]
fn requests_more_rows_than_table_has() -> Result {
    test().run("[date] | last 50 | length").expect_value_eq(1)
}

#[test]
fn gets_last_row_as_list_when_amount_given() -> Result {
    test()
        .run("[1, 2, 3] | last 1 | describe")
        .expect_value_eq("list<int>")
}

#[test]
fn gets_last_bytes() -> Result {
    test()
        .run("(0x[aa bb cc] | last 2) == 0x[bb cc]")
        .expect_value_eq(true)
}

#[test]
fn gets_last_byte() -> Result {
    test().run("0x[aa bb cc] | last").expect_value_eq(204)
}

#[test]
fn gets_last_bytes_from_stream() -> Result {
    test()
        .run("(1..10 | each { 0x[aa bb cc] } | bytes collect | last 2) == 0x[bb cc]")
        .expect_value_eq(true)
}

#[test]
fn gets_last_byte_from_stream() -> Result {
    test()
        .run("1..10 | each { 0x[aa bb cc] } | bytes collect | last")
        .expect_value_eq(204)
}

#[test]
fn last_errors_on_negative_index() -> Result {
    let err = test().run("[1, 2, 3] | last -2").expect_shell_error()?;

    assert_matches!(err, ShellError::NeedsPositiveValue { .. });
    Ok(())
}

#[test]
fn fail_on_non_iterator() -> Result {
    let err = test().run("1 | last").expect_parse_error()?;

    assert_matches!(err, ParseError::InputMismatch(input_type, _) if input_type == "int");
    Ok(())
}

#[test]
fn errors_on_empty_list_when_no_rows_given_in_strict_mode() -> Result {
    let err = test().run("[] | last --strict").expect_shell_error()?;

    assert_matches!(err, ShellError::AccessEmptyContent { .. });
    Ok(())
}

#[test]
fn does_not_error_on_empty_list_when_no_rows_given() -> Result {
    test()
        .run("[] | last | describe")
        .expect_value_eq("nothing")
}

#[test]
fn returns_nothing_on_empty_list_when_no_rows_given() -> Result {
    test().run("[] | last").expect_value_eq(())
}

#[test]
fn returns_d_on_empty_list_when_no_rows_given_with_default() -> Result {
    test()
        .run("[a b] | where $it == 'c' | last | default 'd'")
        .expect_value_eq("d")
}

#[test]
fn wrapping_last_with_optional_null_rows() -> Result {
    let code = "def wraps-last [rows?: int] { [1, 2, 3] | last $rows }; wraps-last";
    test().run(code).expect_value_eq(3)
}

#[test]
fn wrapping_last_with_optional_explicit_rows() -> Result {
    let code = "def wraps-last [rows?: int] { [1, 2, 3] | last $rows }; wraps-last 2 | length";
    test().run(code).expect_value_eq(2)
}

#[test]
fn last_bytes_with_filesize() -> Result {
    let code = "(0x[aa bb cc] | last 2b) == 0x[bb cc]";
    test().run(code).expect_value_eq(true)
}

#[test]
fn last_filesize_list_error() -> Result {
    let err = test().run("[1 2 3] | last 1kb").expect_shell_error()?;
    assert!(
        matches!(err, ShellError::IncompatibleParametersSingle { .. }),
        "expected IncompatibleParametersSingle, got {err:?}"
    );
    Ok(())
}

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

#[rstest]
#[case::list_last(InputKind::List, "last")]
#[case::list_last_n(InputKind::List, "last 2")]
#[case::range_last(InputKind::Range, "last")]
#[case::range_last_n(InputKind::Range, "last 2")]
#[case::list_stream_last(InputKind::ListStream, "last")]
#[case::list_stream_last_n(InputKind::ListStream, "last 2")]
#[case::binary_last(InputKind::Binary, "last")]
#[case::binary_last_n(InputKind::Binary, "last 2")]
fn last_preserves_pipeline_metadata(#[case] input: InputKind, #[case] code: &str) -> Result {
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
