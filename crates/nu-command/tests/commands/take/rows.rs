use nu_test_support::prelude::*;

#[test]
fn rows() -> Result {
    let sample = "
        [[name,   lucky_code];
         [Andrés, 1],
         [JT    , 1],
         [Jason , 2],
         [Yehuda, 1]]";

    let code = format!(
        "
            {sample}
            | take 3
            | get lucky_code
            | math sum
        "
    );

    test().run(code).expect_value_eq(4)
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
    let code = format!(
        "open {} | take 3 | metadata | get content_type? | describe",
        file!(),
    );

    test().run(code).expect_value_eq("nothing")
}
