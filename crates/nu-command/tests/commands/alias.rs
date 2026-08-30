use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::numeric("1234")]
#[case::filesize_like("5gib")]
#[case::hash(r#""te#t""#)]
#[case::caret("^foo")]
fn alias_fails_with_invalid_name(#[case] alias: &str) -> Result {
    let code = format!("alias {alias} = echo 'test'");
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::AliasNotValid(_)));
    Ok(())
}

#[test]
fn alias_fails_with_all_single_word_keyword_names() -> Result {
    for name in nu_parser::single_word_parser_keywords() {
        let code = format!("alias {name} = ls");
        let err = test().run(code).expect_parse_error()?;
        assert!(
            matches!(&err, ParseError::NameIsKeyword(keyword, kind, _) if keyword == name && kind == "alias"),
            "expected NameIsKeyword alias for `{name}`, got {err:?}"
        );
    }
    Ok(())
}

#[test]
fn cant_alias_keyword() -> Result {
    test()
        .run(" alias ou = let ")
        .expect_error_code_eq("nu::parser::cant_alias_keyword")
}

#[test]
fn alias_wont_recurse() -> Result {
    let code = "
        module myspamsymbol {
            export def myfoosymbol [prefix: string, msg: string] {
                $prefix + $msg
            }
        };
        use myspamsymbol myfoosymbol;
        alias myfoosymbol = myfoosymbol 'hello';
        myfoosymbol ' world'
    ";

    test().run(code).expect_value_eq("hello world")
}

// Issue https://github.com/nushell/nushell/issues/8246
#[test]
fn alias_wont_recurse2(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        indoc::indoc! {"
        def eggs [] { spam 'eggs' }
        alias spam = spam 'spam'
    "},
    )?;

    let code = "
        def spam [what: string] { 'spam ' + $what };
        source spam.nu;
        spam
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("spam spam")
}

#[rstest]
#[case::string(" alias spam = 'foo' ")]
#[case::subexpression(" alias spam = ([1 2 3] | length) ")]
#[case::range(" alias spam = 0..12 ")]
fn alias_invalid_expression(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::cant_alias_expression")
}

#[test]
fn alias_if() -> Result {
    test()
        .run(" alias spam = if true { 'spam' } else { 'eggs' }; spam ")
        .expect_value_eq("spam")
}

#[test]
fn alias_match() -> Result {
    test()
        .run(" alias spam = match 3 { 1..10 => 'yes!' }; spam ")
        .expect_value_eq("yes!")
}

// Issue https://github.com/nushell/nushell/issues/8103
#[rstest]
#[case::backticks("alias `foo bar` = echo 'test'; foo bar")]
#[case::single_quotes("alias 'foo bar' = echo 'test'; foo bar")]
#[case::double_quotes(r#"alias "foo bar" = echo 'test'; foo bar"#)]
fn alias_multiword_name(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("test")
}

#[test]
fn alias_ordering() -> Result {
    test()
        .run("alias bar = echo; def echo [] { 'dummy echo' }; bar 'foo'")
        .expect_value_eq("foo")
}

#[test]
fn alias_default_help() -> Result {
    let actual: String =
        test().run("alias teapot = echo 'I am a beautiful teapot'; help teapot")?;
    // There must be at least one line of help
    assert!(actual.starts_with("Alias for `echo 'I am a beautiful teapot'`"));
    Ok(())
}

#[test]
fn export_alias_with_overlay_use_works() -> Result {
    test()
        .run("export alias teapot = overlay use")
        .expect_value_eq(())
}

#[test]
fn alias_flag() -> Result {
    test().run("alias si = stor import").expect_value_eq(())
}
