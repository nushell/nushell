use nu_protocol::{
    IntRange, IntoPipelineData, ListStream, PipelineData, PipelineMetadata, Range, Signals, Span,
    Value, ast::RangeInclusion,
};
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
use rstest::rstest;

#[test]
fn gets_the_last_row() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | last 1 | get name.0 | str trim"
    );

    assert_eq!(actual.out, "utf16.ini");
}

#[test]
fn gets_last_rows_by_amount() {
    Playground::setup("last_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(cwd: dirs.test(), "ls | last 3 | length");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_last_row_when_no_amount_given() {
    Playground::setup("last_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        // FIXME: We should probably change last to return a one row table instead of a record here
        let actual = nu!(cwd: dirs.test(), "ls | last | values | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn requests_more_rows_than_table_has() {
    let actual = nu!("[date] | last 50 | length");

    assert_eq!(actual.out, "1");
}

#[test]
fn gets_last_row_as_list_when_amount_given() {
    let actual = nu!("[1, 2, 3] | last 1 | describe");

    assert_eq!(actual.out, "list<int>");
}

#[test]
fn gets_last_bytes() {
    let actual = nu!("(0x[aa bb cc] | last 2) == 0x[bb cc]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_last_byte() {
    let actual = nu!("0x[aa bb cc] | last");

    assert_eq!(actual.out, "204");
}

#[test]
fn gets_last_bytes_from_stream() {
    let actual = nu!("(1..10 | each { 0x[aa bb cc] } | bytes collect | last 2) == 0x[bb cc]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_last_byte_from_stream() {
    let actual = nu!("1..10 | each { 0x[aa bb cc] } | bytes collect | last");

    assert_eq!(actual.out, "204");
}

#[test]
fn last_errors_on_negative_index() {
    let actual = nu!("[1, 2, 3] | last -2");

    assert!(actual.err.contains("use a positive value"));
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | last");

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn errors_on_empty_list_when_no_rows_given_in_strict_mode() {
    let actual = nu!("[] | last --strict");
    assert!(actual.err.contains("index too large"));
}

#[test]
fn does_not_error_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | last | describe");

    assert!(actual.out.contains("nothing"));
}

#[test]
fn returns_nothing_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | last");

    assert_eq!(actual.out, "");
}

#[test]
fn returns_d_on_empty_list_when_no_rows_given_with_default() {
    let actual = nu!("[a b] | where $it == 'c' | last | default 'd'");

    assert_eq!(actual.out, "d");
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
