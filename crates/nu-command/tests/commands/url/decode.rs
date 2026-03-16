use nu_test_support::prelude::*;

#[test]
fn url_decode_simple() -> Result {
    test().run("'a%20b' | url decode").expect_value_eq("a b")
}

#[test]
fn url_decode_special_characters() -> Result {
    let code = "'%21%40%23%24%25%C2%A8%26%2A%2D%2B%3B%2C%7B%7D%5B%5D%28%29' | url decode";
    test().run(code).expect_value_eq("!@#$%¨&*-+;,{}[]()")
}

#[test]
fn url_decode_error_invalid_utf8() -> Result {
    let err = test().run("'%99' | url decode").expect_error()?;
    match err {
        ShellError::GenericError { error, msg, .. } => {
            assert_eq!(error, "Failed to decode string");
            assert_contains("invalid utf-8 sequence", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}
