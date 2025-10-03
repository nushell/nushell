use nu_test_support::nu;

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
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
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn to_xml_error_unknown_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {tag: a bad_column: b} | to xml
    "#);

    assert!(actual.err.contains("Invalid column \"bad_column\""));
}

#[test]
fn to_xml_error_no_tag() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {attributes: {a: b c: d}} | to xml
    "#);

    assert!(actual.err.contains("Tag missing"));
}

#[test]
fn to_xml_error_tag_not_string() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {tag: 1 attributes: {a: b c: d}} | to xml
    "#);

    assert!(actual.err.contains("not a string"));
}

#[test]
fn to_xml_partial_escape() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {
            tag: a
            attributes: { a: "'a'\\" }
            content: [ `'"qwe\` ]
        } | to xml --partial-escape
    "#);
    assert_eq!(actual.out, r#"<a a="'a'\">'"qwe\</a>"#);
}

#[test]
fn to_xml_pi_comment_not_escaped() {
    // PI and comment content should not be escaped
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {
            tag: a
            content: [
                {tag: ?qwe content: `"'<>&`}
                {tag: ! content: `"'<>&`}
            ]
        } | to xml
    "#);
    assert_eq!(actual.out, r#"<a><?qwe "'<>&?><!--"'<>&--></a>"#);
}

#[test]
fn to_xml_self_closed() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {
            tag: root
            content: [
                [tag attributes content];
                [a null null]
                [b {e: r} null]
                [c {t: y} []]
            ]
        } | to xml --self-closed
    "#);
    assert_eq!(actual.out, r#"<root><a/><b e="r"/><c t="y"/></root>"#);
}
