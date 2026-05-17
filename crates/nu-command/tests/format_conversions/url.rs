use nu_test_support::prelude::*;

#[test]
fn can_encode_and_decode_urlencoding() -> Result {
    let code = "
        open sample.url
        | url build-query
        | from url
        | get cheese
    ";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("comté")
}
