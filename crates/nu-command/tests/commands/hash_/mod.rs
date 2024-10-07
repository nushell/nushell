use nu_test_support::{nu, pipeline};

#[test]
fn base64_defaults_to_encoding_with_standard_character_type() {
    let actual = nu!(r#"
        echo 'username:password' | encode base64
        "#);

    assert_eq!(actual.out, "dXNlcm5hbWU6cGFzc3dvcmQ=");
}

#[test]
fn base64_defaults_to_encoding_with_nopad() {
    let actual = nu!(r#"
        echo 'username:password' | encode base64 --nopad
        "#);

    assert_eq!(actual.out, "dXNlcm5hbWU6cGFzc3dvcmQ");
}

#[test]
fn base64_encode_url_safe() {
    let actual = nu!(r#"
        0x[BE EE FF] | encode base64 --url
        "#);

    assert_eq!(actual.out, "vu7_");
}

#[test]
fn md5_works_with_file() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open sample.db --raw | hash md5
        "#
        )
    );

    assert_eq!(actual.out, "4de97601d232c427977ef11db396c951");
}

#[test]
fn sha256_works_with_file() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open sample.db --raw | hash sha256
        "#
        )
    );

    assert_eq!(
        actual.out,
        "2f5050e7eea415c1f3d80b5d93355efd15043ec9157a2bb167a9e73f2ae651f2"
    );
}
