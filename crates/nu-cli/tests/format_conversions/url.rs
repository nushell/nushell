use nu_test_support::{nu, pipeline};

#[test]
fn can_encode_and_decode_urlencoding() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
            r#"
                open sample.url
                | to url
                | from url
                | get cheese
            "#
    ));

    assert_eq!(actual.out, "comté");
}
