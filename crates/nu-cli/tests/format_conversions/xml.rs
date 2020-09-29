use nu_test_support::{nu, pipeline};

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open jonathan.xml
            | to xml
            | from xml
            | get rss.children.channel.children.0.item.children.0.guid.attributes.isPermaLink
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "true");
}
