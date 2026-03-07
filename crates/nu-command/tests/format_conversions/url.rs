use nu_test_support::prelude::*;

#[test]
fn can_encode_and_decode_urlencoding() -> Result {
    let code = r#"
        open sample.url
        | url build-query
        | from url
        | get cheese
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "comté");
    Ok(())
}
