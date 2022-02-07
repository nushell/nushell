use nu_test_support::{nu, pipeline};

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open jonathan.xml
            | to xml
            | from xml
<<<<<<< HEAD
            | get rss.children.channel.children.0.item.children.0.guid.attributes.isPermaLink
=======
            | get rss.children.channel.children.0.3.item.children.guid.4.attributes.isPermaLink
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "true");
}
