use nu_protocol::{IntoPipelineData, Span};
use nu_test_support::prelude::*;

#[test]
fn rows() -> Result {
    let sample = "
        [[name,   lucky_code];
         [Andrés, 1],
         [JT    , 1],
         [Jason , 2],
         [Yehuda, 1]]";

    let code = "
        from nuon
        | take 3
        | get lucky_code
        | math sum
    ";

    test().run_with_data(code, sample).expect_value_eq(4)
}

#[test]
fn rows_with_no_arguments_should_lead_to_error() -> Result {
    let code = "[1 2 3] | take";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::MissingPositional(..)));
    Ok(())
}

#[test]
fn fails_on_string() -> Result {
    let code = r#""foo bar" | take 2"#;
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch(..)));
    Ok(())
}

#[test]
fn takes_bytes() -> Result {
    let code = "(0x[aa bb cc] | take 2) == 0x[aa bb]";
    test().run(code).expect_value_eq(true)
}

#[test]
fn takes_bytes_from_stream() -> Result {
    let code = "(1.. | each { 0x[aa bb cc] } | bytes collect | take 2) == 0x[aa bb]";
    test().run(code).expect_value_eq(true)
}

#[test]
// covers a situation where `take` used to behave strangely on list<binary> input
fn works_with_binary_list() -> Result {
    let code = "
            ([0x[01 11]] | take 1 | get 0) == 0x[01 11]
        ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn takes_bytes_and_drops_content_type() -> Result {
    let content_type = test()
        .run_raw_with_data(
            "open $in | take 3",
            (file!()).into_value(Span::test_data()).into_pipeline_data(),
        )?
        .take_metadata()
        .and_then(|md| md.content_type);

    assert!(content_type.is_none());
    Ok(())
}
