use nu_protocol::Span;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
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

/// A pipe or redirection in an alias is a parse error at the operator, not an argument.
#[rstest]
#[case::pipe("alias spam = echo 1 | str length", "|", "pipeline")]
#[case::pipe_then_newline("alias spam = echo 1 |\n    str length", "|", "pipeline")]
#[case::stderr_pipe("alias spam = ls e>| lines", "e>|", "pipeline")]
#[case::redirection("alias spam = echo 1 o> out.txt", "o>", "redirection")]
#[case::append("alias spam = echo 1 out>> out.txt", "out>>", "redirection")]
#[case::export_alias(
    "module m { export alias spam = echo 1 | str length }",
    "|",
    "pipeline"
)]
fn alias_rejects_pipe_or_redirection(
    #[case] code: &str,
    #[case] operator: &str,
    #[case] what: &str,
) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::CantAliasExpression(ref found, span)
            if found == what && span.end - span.start == operator.len()
    );
    Ok(())
}

/// A `|` inside quotes or parentheses is part of a word, so the alias keeps it.
#[rstest]
#[case::quoted("alias spam = echo '|' x; spam", test_value!(["|", "x"]))]
#[case::subexpression("alias spam = echo ([1 2 3] | length); spam", test_value!(3))]
fn alias_keeps_pipe_inside_a_word(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
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

/// An alias that ends with a flag that takes a value parses like its expansion followed by the
/// words after the alias, so the first of those words is the flag's value.
#[rstest]
#[case::long_flag("alias hex = into int --radix; 'ff' | hex 16", test_value!(255))]
#[case::short_flag("alias hex = into int -r; 'ff' | hex 16", test_value!(255))]
#[case::short_flag_group(
    "alias myfind = find -nr; ['file.txt' 'x'] | myfind 'txt$'",
    test_value!(["file.txt"])
)]
#[case::more_flags_after(
    "alias myfind = find -nr; ['FILE.TXT' 'x'] | myfind 'txt$' -i",
    test_value!(["FILE.TXT"])
)]
#[case::alias_of_alias(
    "alias hex = into int --radix; alias hex2 = hex; 'ff' | hex2 16",
    test_value!(255)
)]
#[case::switch_is_not_filled("alias rep = str replace -a; 'aaa' | rep a b", test_value!("bbb"))]
fn alias_ending_with_flag_takes_value_from_use(
    #[case] code: &str,
    #[case] expected: Value,
) -> Result {
    test().run(code).expect_value_eq(expected)
}

/// `--help` after such an alias still shows help rather than becoming the flag's value.
#[rstest]
#[case::long("alias myfind = find -r; [] | myfind --help")]
#[case::short("alias myfind = find -r; [] | myfind -h")]
fn alias_ending_with_flag_still_shows_help(#[case] code: &str) -> Result {
    let help: String = test().run(code)?;
    assert_contains("Search for terms in the input data.", help);
    Ok(())
}

/// Without a word to fill it, the flag's missing value is reported at the alias's name where it
/// is used, not where it is defined.
#[test]
fn alias_ending_with_flag_reports_missing_value_at_use() -> Result {
    let code = "alias hex = into int --radix; 'ff' | hex";
    let mut tester = test();
    let entry_start = tester.engine_state.next_span_start();
    let err = tester.run(code).expect_parse_error()?;
    let use_start = entry_start + code.rfind("hex").expect("code uses the alias");
    assert_matches!(
        err,
        ParseError::MissingFlagParam(_, span) if span == Span::new(use_start, use_start + 3)
    );
    Ok(())
}

/// A runtime error about the filled value points at the value. The flag's name comes from the
/// alias definition, in an earlier entry here, so a span covering both would reach across
/// entries and the error couldn't show its source.
#[test]
fn alias_ending_with_flag_reports_value_error_at_use() -> Result {
    let mut tester = test();
    let () = tester.run("alias hex = into int --radix")?;
    let err = tester.run("'ff' | hex 99").expect_shell_error()?;
    let ShellError::TypeMismatch { span, .. } = err else {
        panic!("expected a radix TypeMismatch, got {err:?}");
    };
    assert_eq!(tester.engine_state.get_span_contents(span), b"99");
    Ok(())
}

/// An alias defined in one entry, like one from `config.nu`, fills the flag in a later entry.
#[test]
fn alias_ending_with_flag_used_in_later_entry() -> Result {
    let mut tester = test();
    let () = tester.run("alias hex = into int --radix")?;
    tester.run("'ff' | hex 16").expect_value_eq(255)?;
    let err = tester.run("'ff' | hex").expect_parse_error()?;
    assert_matches!(err, ParseError::MissingFlagParam(..));
    Ok(())
}
