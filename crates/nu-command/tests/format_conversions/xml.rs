use nu_test_support::prelude::*;

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() -> Result {
    let code = "
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
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("true")
}

#[test]
fn to_xml_error_unknown_column() -> Result {
    let code = "{tag: a bad_column: b} | to xml";

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    let ShellError::CantConvert {
        help: Some(help), ..
    } = err
    else {
        return Err(err.into());
    };
    assert_contains("Invalid column \"bad_column\"", help);
    Ok(())
}

#[test]
fn to_xml_error_no_tag() -> Result {
    let code = "{attributes: {a: b c: d}} | to xml";

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    let ShellError::CantConvert {
        help: Some(help), ..
    } = err
    else {
        return Err(err.into());
    };
    assert_contains("Tag missing", help);
    Ok(())
}

#[test]
fn to_xml_error_tag_not_string() -> Result {
    let code = "{tag: 1 attributes: {a: b c: d}} | to xml";

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    let ShellError::CantConvert {
        help: Some(help), ..
    } = err
    else {
        return Err(err.into());
    };
    assert_contains("not a string", help);
    Ok(())
}

#[test]
fn to_xml_partial_escape() -> Result {
    let code = r#"
        {
            tag: a
            attributes: { a: "'a'\\" }
            content: [ `"'"qwe\` ]
        } | to xml --partial-escape
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(r#"<a a="'a'\">"'"qwe\</a>"#)
}

#[test]
fn to_xml_pi_comment_not_escaped() -> Result {
    // PI and comment content should not be escaped
    let code = r#"
        {
            tag: a
            content: [
                {tag: ?qwe content: `"'<>&`}
                {tag: ! content: `"'<>&`}
            ]
        } | to xml
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(r#"<a><?qwe "'<>&?><!--"'<>&--></a>"#)
}

#[test]
fn to_xml_self_closed() -> Result {
    let code = "
        {
            tag: root
            content: [
                [tag attributes content];
                [a null null]
                [b {e: r} null]
                [c {t: y} []]
            ]
        } | to xml --self-closed
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(r#"<root><a/><b e="r"/><c t="y"/></root>"#)
}

#[test]
fn to_xml_negative_indentation() -> Result {
    test()
        .run("{tag: a} | to xml --indent -1")
        .expect_error_code_eq("nu::shell::needs_positive_value")
}

#[test]
fn from_xml_to_xml_round_trips_default_namespace_and_xml_prefix() -> Result {
    let code = r#"
        '<svg xmlns="http://www.w3.org/2000/svg" width="1"><text xml:space="preserve">a</text></svg>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1"><text xml:space="preserve">a</text></svg>"#)
}

#[test]
fn from_xml_to_xml_round_trips_prefixed_rss_element() -> Result {
    let code = r#"
        '<rss xmlns:dc="http://purl.org/dc/elements/1.1/"><item><dc:creator>x</dc:creator></item></rss>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<rss xmlns:dc="http://purl.org/dc/elements/1.1/"><item><dc:creator>x</dc:creator></item></rss>"#)
}

#[test]
fn from_xml_to_xml_keeps_default_namespace_element_unprefixed() -> Result {
    let code = r#"
        '<a xmlns:p="u" xmlns="u"><b>t</b></a>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<a xmlns:p="u" xmlns="u"><b>t</b></a>"#)
}

#[test]
fn from_xml_to_xml_keeps_attribute_prefix_when_default_namespace_is_also_bound() -> Result {
    let code = r#"
        '<a xmlns="u" xmlns:p="u"><p:b p:k="1">t</p:b></a>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<a xmlns="u" xmlns:p="u"><p:b p:k="1">t</p:b></a>"#)
}

#[test]
fn from_xml_to_xml_regression_prefixed_then_default_child() -> Result {
    let code = r#"
        '<a xmlns:p="u"><p:b>t</p:b><c xmlns="u"></c></a>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<a xmlns:p="u"><p:b>t</p:b><c xmlns="u"></c></a>"#)
}

#[test]
fn from_xml_default_output_drops_prefixes_and_declarations() -> Result {
    let code = r#"
        '<svg xmlns="http://www.w3.org/2000/svg" width="1"><text xml:space="preserve">a</text></svg>'
        | from xml
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<svg width="1"><text space="preserve">a</text></svg>"#)
}

#[test]
fn from_xml_default_output_uses_local_tag_names() -> Result {
    let code = r#"
        '<rss xmlns:dc="http://purl.org/dc/elements/1.1/"><item><dc:creator>x</dc:creator></item></rss>'
        | from xml
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq("<rss><item><creator>x</creator></item></rss>")
}

#[test]
fn from_xml_namespaces_keeps_svg_prefix_when_uri_is_also_default() -> Result {
    let code = r#"
        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:svg="http://www.w3.org/2000/svg"><svg:rect width="1"/></svg>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:svg="http://www.w3.org/2000/svg"><svg:rect width="1"></svg:rect></svg>"#)
}

#[test]
fn from_xml_namespaces_filter_matches_written_prefixed_tag() -> Result {
    let code = r#"
        '<xs:schema xmlns="http://www.w3.org/2001/XMLSchema" xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="note" type="xs:string"/></xs:schema>'
        | from xml --namespaces
        | get content
        | where tag == 'xs:element'
        | length
    "#;

    test().run(code).expect_value_eq(1)
}

#[test]
fn from_xml_namespaces_round_trips_second_prefix_for_same_uri() -> Result {
    let code = r#"
        '<a xmlns:p="u" xmlns:q="u"><q:b q:k="1"/></a>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<a xmlns:p="u" xmlns:q="u"><q:b q:k="1"></q:b></a>"#)
}

#[test]
fn from_xml_namespaces_round_trips_prefix_bound_on_ancestor() -> Result {
    let code = r#"
        '<a xmlns:dc="X"><b xmlns:x="X"><dc:t/></b></a>'
        | from xml --namespaces
        | to xml
    "#;

    test()
        .run(code)
        .expect_value_eq(r#"<a xmlns:dc="X"><b xmlns:x="X"><dc:t></dc:t></b></a>"#)
}
