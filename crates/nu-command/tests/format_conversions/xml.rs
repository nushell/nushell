use nu_test_support::prelude::*;

#[test]
fn table_to_xml_text_and_from_xml_text_back_into_table() -> Result {
    let code = r#"
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
    "#;

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
    let code = r#"
        {
            tag: root
            content: [
                [tag attributes content];
                [a null null]
                [b {e: r} null]
                [c {t: y} []]
            ]
        } | to xml --self-closed
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(r#"<root><a/><b e="r"/><c t="y"/></root>"#)
}
