use nu_protocol::{
    IntRange, IntoPipelineData, ListStream, PipelineData, PipelineMetadata, Range, Signals, Span,
    Value, ast::RangeInclusion,
};
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
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
fn gets_first_rows_by_amount() {
    Playground::setup("first_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(cwd: dirs.test(), "ls | first 3 | length");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_all_rows_if_amount_higher_than_all_rows() {
    Playground::setup("first_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), "ls | first 99 | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_when_no_amount_given() {
    Playground::setup("first_test_3", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        // FIXME: We should probably change first to return a one row table instead of a record here
        let actual = nu!(cwd: dirs.test(), "ls | first | values | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_as_list_when_amount_given() {
    let actual = nu!("[1, 2, 3] | first 1 | describe");

    assert_eq!(actual.out, "list<int>");
}

#[test]
fn gets_first_bytes() {
    let actual = nu!("(0x[aa bb cc] | first 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_first_byte() {
    let actual = nu!("0x[aa bb cc] | first");

    assert_eq!(actual.out, "170");
}

#[test]
fn gets_first_bytes_from_stream() {
    let actual = nu!("(1.. | each { 0x[aa bb cc] } | bytes collect | first 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_first_byte_from_stream() {
    let actual = nu!("1.. | each { 0x[aa bb cc] } | bytes collect | first");

    assert_eq!(actual.out, "170");
}

#[test]
// covers a situation where `first` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!("([0x[01 11]] | first) == 0x[01 11]");

    assert_eq!(actual.out, "true");
}

#[test]
fn errors_on_negative_rows() {
    let actual = nu!("[1, 2, 3] | first -10");

    assert!(actual.err.contains("use a positive value"));
}

#[test]
fn does_not_error_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | first | describe");

    assert!(actual.out.contains("nothing"));
}

#[test]
fn error_on_empty_list_when_no_rows_given_in_strict_mode() {
    let actual = nu!("[] | first --strict | describe");

    assert!(actual.err.contains("index too large"));
}

#[test]
fn gets_first_bytes_and_drops_content_type() {
    let actual = nu!(format!(
        "open {} | first 3 | metadata | get content_type? | describe",
        file!(),
    ));
    assert_eq!(actual.out, "nothing");
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
