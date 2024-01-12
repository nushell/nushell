use nu_test_support::{nu, pipeline};

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open jt.xml
            | to xml
            | from xml
            | get content
            | where tag == channel
            | get content
            | flatten
            | where tag == item
            | get content
            | flatten
            | where tag == guid
            | get 0.attributes.isPermaLink
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_xml_error_unknown_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            {tag: a bad_column: b} | to xml
        "#
    ));

    assert!(actual.err.contains("Invalid column \"bad_column\""));
}

#[test]
fn to_xml_error_no_tag() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            {attributes: {a: b c: d}} | to xml
        "#
    ));

    assert!(actual.err.contains("Tag missing"));
}

#[test]
fn to_xml_error_tag_not_string() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            {tag: 1 attributes: {a: b c: d}} | to xml
        "#
    ));

    assert!(actual.err.contains("not a string"));
}
