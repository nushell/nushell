use nu_test_support::nu;

#[test]
fn can_encode_and_decode_urlencoding() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample.url
        | url build-query
        | from url
        | get cheese
    "#);

    assert_eq!(actual.out, "comté");
}
