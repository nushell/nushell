use nu_test_support::{nu, pipeline};

#[test]
fn base64_defaults_to_encoding_with_standard_character_type() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 'username:password' | hash base64 
        "#
        )
    );

    assert_eq!(actual.out, "dXNlcm5hbWU6cGFzc3dvcmQ=");
}

#[test]
fn base64_encode_characterset_binhex() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 'username:password' | hash base64 --character_set binhex --encode
        "#
        )
    );

    assert_eq!(actual.out, "F@0NEPjJD97kE\'&bEhFZEP3");
}

#[test]
fn error_when_invalid_character_set_given() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 'username:password' | hash base64 --character_set 'this is invalid' --encode
        "#
        )
    );

    assert!(actual
        .err
        .contains("this is invalid is not a valid character-set"));
}

#[test]
fn base64_decode_characterset_binhex() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "F@0NEPjJD97kE'&bEhFZEP3" | hash base64 --character_set binhex --decode
        "#
        )
    );

    assert_eq!(actual.out, "username:password");
}

#[test]
fn error_invalid_decode_value() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "this should not be a valid encoded value" | hash base64 --character_set url-safe --decode
        "#
        )
    );

    assert!(actual
        .err
        .contains("invalid base64 input for character set url-safe"));
}

#[test]
fn error_use_both_flags() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 'username:password' | hash base64 --encode --decode
        "#
        )
    );

    assert!(actual
        .err
        .contains("only one of --decode and --encode flags can be used"));
}
