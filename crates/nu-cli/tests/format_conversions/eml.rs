use nu_test_support::{nu, pipeline};

const TEST_CWD: &str = "tests/fixtures/formats";

// The To field in this email is just "to@example.com", which gets parsed out as the Address. The Name is empty.
#[test]
fn from_eml_get_to_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get To
            | get Address
        "#
        )
    );

    assert_eq!(actual.out, "to@example.com");

    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get To
            | get Name
        "#
        )
    );

    assert_eq!(actual.out, "");
}

// The Reply-To field in this email is "replyto@example.com" <replyto@example.com>, meaning both the Name and Address values are identical.
#[test]
fn from_eml_get_replyto_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Reply-To
            | get Address
        "#
        )
    );

    assert_eq!(actual.out, "replyto@example.com");

    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Reply-To
            | get Name
        "#
        )
    );

    assert_eq!(actual.out, "replyto@example.com");
}

#[test]
fn from_eml_get_subject_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Subject
        "#
        )
    );

    assert_eq!(actual.out, "Test Message");
}

#[test]
fn from_eml_get_another_header_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get MIME-Version
        "#
        )
    );

    assert_eq!(actual.out, "1.0");
}
