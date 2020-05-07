use nu_test_support::{nu, pipeline};

const TEST_CWD: &str = "tests/fixtures/formats";

// The To field in this email is just "username@domain.com", which gets parsed out as the Address. The Name is empty.
#[test]
fn from_eml_get_to_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get To
            | get Address
            | echo $it
        "#
        )
    );

    assert_eq!(actual.out, "username@domain.com");

    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get To
            | get Name
            | echo $it
        "#
        )
    );

    assert_eq!(actual.out, "");
}

// The Reply-To field in this email is "aw-confirm@ebay.com" <aw-confirm@ebay.com>, meaning both the Name and Address values are identical.
#[test]
fn from_eml_get_replyto_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Reply-To
            | get Address
            | echo $it
        "#
        )
    );

    assert_eq!(actual.out, "aw-confirm@ebay.com");

    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Reply-To
            | get Name
            | echo $it
        "#
        )
    );

    assert_eq!(actual.out, "aw-confirm@ebay.com");
}

// The Reply-To field in this email is "aw-confirm@ebay.com" <aw-confirm@ebay.com>, meaning both the Name and Address values are identical.
#[test]
fn from_eml_get_subject_field() {
    let actual = nu!(
        cwd: TEST_CWD,
        pipeline(
            r#"
            open sample.eml
            | get Subject
            | echo $it
        "#
        )
    );

    assert_eq!(actual.out, "Billing Issues");
}
