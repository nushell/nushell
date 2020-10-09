use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn position_function_in_predicate() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "<?xml version="1.0" encoding="UTF-8"?><a><b/><b/></a>" | from xml | to xml | xpath "count(//a/*[position() = 2])"
        "#
    ));

    assert_eq!(actual.out, "1.0000");
}

#[test]
fn functions_implicitly_coerce_argument_types() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "<?xml version="1.0" encoding="UTF-8"?><a>true</a>" | from xml | to xml | xpath "count(//*[contains(., true)])"
        "#
    ));

    assert_eq!(actual.out, "1.0000");
}

#[test]
fn find_guid_permilink_is_true() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open jonathan.xml
            | to xml
            | xpath '//guid/@isPermaLink'
        "#
    ));

    assert_eq!(actual.out, "true");
}
