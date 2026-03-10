use nu_test_support::prelude::*;

#[test]
fn base64_defaults_to_encoding_with_standard_character_type() -> Result {
    let code = r#"
        echo 'username:password' | encode base64
        "#;

    test().run(code).expect_value_eq("dXNlcm5hbWU6cGFzc3dvcmQ=")
}

#[test]
fn base64_defaults_to_encoding_with_nopad() -> Result {
    let code = r#"
        echo 'username:password' | encode base64 --nopad
        "#;

    test().run(code).expect_value_eq("dXNlcm5hbWU6cGFzc3dvcmQ")
}

#[test]
fn base64_decode_value() -> Result {
    let code = r#"
        echo 'YWJjeHl6' | decode base64 | decode
        "#;

    test().run(code).expect_value_eq("abcxyz")
}

#[test]
fn base64_decode_with_nopad() -> Result {
    let code = r#"
        echo 'R29vZCBsdWNrIHRvIHlvdQ' | decode base64 --nopad | decode
        "#;

    test().run(code).expect_value_eq("Good luck to you")
}

#[test]
fn base64_decode_with_url() -> Result {
    let code = r#"
        echo 'vu7_' | decode base64 --url | decode
        "#;

    test().run(code).expect_value_eq("¾îÿ")
}

#[test]
fn error_invalid_decode_value() -> Result {
    let code = r#"
        echo "this should not be a valid encoded value" | decode base64
        "#;

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn md5_works_with_file() -> Result {
    let code = r#"
    open sample.db --raw | hash md5
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("4de97601d232c427977ef11db396c951")
}

#[test]
fn sha256_works_with_file() -> Result {
    let code = r#"
    open sample.db --raw | hash sha256
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("2f5050e7eea415c1f3d80b5d93355efd15043ec9157a2bb167a9e73f2ae651f2")
}
