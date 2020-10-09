use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn count_columns_in_cal_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal | count -c
        "#
    ));

    assert_eq!(actual.out, "7");
}

#[test]
fn count_columns_no_rows() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [] | count -c
        "#
    ));

    assert_eq!(actual.out, "0");
}

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
