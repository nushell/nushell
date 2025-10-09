use nu_test_support::nu;

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
fn base64_decode_value() {
    let actual = nu!(r#"
        echo 'YWJjeHl6' | decode base64 | decode
        "#);

    assert_eq!(actual.out, "abcxyz");
}

#[test]
fn base64_decode_with_nopad() {
    let actual = nu!(r#"
        echo 'R29vZCBsdWNrIHRvIHlvdQ' | decode base64 --nopad | decode
        "#);

    assert_eq!(actual.out, "Good luck to you");
}

#[test]
fn base64_decode_with_url() {
    let actual = nu!(r#"
        echo 'vu7_' | decode base64 --url | decode
        "#);

    assert_eq!(actual.out, "¾îÿ");
}

#[test]
fn error_invalid_decode_value() {
    let actual = nu!(r#"
        echo "this should not be a valid encoded value" | decode base64
        "#);

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}

#[test]
fn md5_works_with_file() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open sample.db --raw | hash md5
    "#);

    assert_eq!(actual.out, "4de97601d232c427977ef11db396c951");
}

#[test]
fn sha256_works_with_file() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open sample.db --raw | hash sha256
    "#);

    assert_eq!(
        actual.out,
        "2f5050e7eea415c1f3d80b5d93355efd15043ec9157a2bb167a9e73f2ae651f2"
    );
}
