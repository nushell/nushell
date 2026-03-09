use nu_test_support::prelude::*;

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
    let code = format!(
        "open {} | skip 3 | metadata | get content_type? | describe",
        file!(),
    );
    test().run(code).expect_value_eq("nothing")
}
