use nu_test_support::{
    fs::Stub::{EmptyFile, FileWithContentToBeTrimmed},
    prelude::*,
};
use rstest::rstest;

#[test]
fn def_with_trailing_comma() -> Result {
    test()
        .run("def test-command [ foo: int, ] { $foo }; test-command 1")
        .expect_value_eq(1)
}

#[rstest]
#[case::command(
    "def_with_comment",
    "
        #My echo
        export def e [arg] {echo $arg}
    ",
    "My echo"
)]
#[case::parameter(
    "def_with_param_comment",
    "
        export def e [
        param:string #My cool attractive param
        ] {echo $param};
    ",
    "My cool attractive param"
)]
fn def_help_includes_comments(
    #[case] playground: &str,
    #[case] fixture: &str,
    #[case] expected: &str,
) -> Result {
    Playground::setup(playground, |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed("def_test", fixture)]);

        let actual: String = test().cwd(dirs.test()).run("use def_test e; help e")?;
        assert_contains(expected, actual);
        Ok(())
    })
}

#[rstest]
#[case::bracket_after_name("def test-command[] {}", "no space")]
#[case::paren_after_name("def --env test-command() {}", "no space")]
#[case::multiple_short_flags(
    "def test-command [ --long(-l)(-o) ] {}",
    "only one short flag alternative"
)]
#[case::comma_before_alternative_short_flag(
    "def test-command [ --long, (-l) ] {}",
    "parameter or flag"
)]
#[case::comma_before_equals("def test-command [ foo, = 1 ] {}", "parameter or flag")]
#[case::colon_before_equals("def test-command [ foo: = 1 ] {}", "type")]
#[case::comma_before_colon("def test-command [ foo, : int ] {}", "parameter or flag")]
#[case::multiple_colons("def test-command [ foo::int ] {}", "type")]
#[case::multiple_types("def test-command [ foo:int:string ] {}", "parameter or flag")]
#[case::trailing_colon("def test-command [ foo: int: ] {}", "parameter or flag")]
#[case::trailing_default_value("def test-command [ foo: int = ] {}", "default value")]
#[case::multiple_commas("def test-command [ foo,,bar ] {}", "parameter or flag")]
fn def_syntax_errors(#[case] code: &str, #[case] expected: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    match err {
        ParseError::Expected(got, _) => assert_eq!(expected, got),
        ParseError::ExpectedWithStringMsg(got, _) => assert_eq!(expected, got),
        ParseError::InternalError(got, _) => assert_contains(expected, got),
        got => assert_contains(expected, got.to_string()),
    }
    Ok(())
}

#[rstest]
#[case::numeric("1234")]
#[case::filesize_like("5gib")]
#[case::caret("^foo")]
fn def_fails_with_invalid_name(#[case] alias: &str) -> Result {
    let code = format!("def {alias} = echo 'test'");
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::CommandDefNotValid(_)));
    Ok(())
}

#[track_caller]
fn assert_name_is_keyword_command(err: &ParseError, name: &str) {
    assert!(
        matches!(err, ParseError::NameIsKeyword(keyword, kind, _) if keyword == name && kind == "command"),
        "expected NameIsKeyword command for `{name}`, got {err:?}"
    );
}

#[test]
fn def_fails_with_all_single_word_keyword_names() -> Result {
    // Driven by the canonical keyword tables so new keywords stay covered.
    for name in nu_parser::single_word_parser_keywords() {
        let code = format!("def {name} [] {{}}");
        let err = test().run(code).expect_parse_error()?;
        assert_name_is_keyword_command(&err, name);
    }
    Ok(())
}

#[test]
fn def_keyword_name_does_not_panic_on_subsequent_parse() -> Result {
    // Regression for the original panic: defining a command named `def` used to
    // shadow the keyword, then re-parsing incomplete `def` input (REPL / `ast`)
    // hit `expect("def call already checked")`. Shadowing is now rejected first.
    let err = test()
        .run(r#"def def [] {}; ast "def""#)
        .expect_parse_error()?;
    assert_name_is_keyword_command(&err, "def");
    Ok(())
}

#[test]
fn export_def_fails_with_keyword_name() -> Result {
    let err = test()
        .run("module m { export def def [] {} }")
        .expect_parse_error()?;
    assert_name_is_keyword_command(&err, "def");
    Ok(())
}

#[test]
fn extern_fails_with_keyword_name() -> Result {
    let err = test().run("extern def []").expect_parse_error()?;
    assert_name_is_keyword_command(&err, "def");
    Ok(())
}

#[test]
fn non_keyword_command_names_are_still_allowed() -> Result {
    // Built-in *commands* (not parser keywords) may still be shadowed.
    test()
        .run("def ls [] { 'shadowed' }; ls")
        .expect_value_eq("shadowed")
}

#[rstest]
#[case::typed(
    "def_with_list",
    "
        def e [
        param: list
        ] {echo $param};
    ",
    "source def_test; e [one]"
)]
#[case::default(
    "def_with_default_list",
    "
        def f [
        param: list = [one]
        ] {echo $param};
    ",
    "source def_test; f"
)]
fn def_with_list(#[case] playground: &str, #[case] fixture: &str, #[case] code: &str) -> Result {
    Playground::setup(playground, |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed("def_test", fixture)]);

        test().cwd(dirs.test()).run(code).expect_value_eq(["one"])
    })
}

#[test]
fn def_with_paren_params() -> Result {
    test()
        .run("def foo (x: int, y: int) { $x + $y }; foo 1 2")
        .expect_value_eq(3)
}

#[rstest]
#[case::positional("def foo [x: any = null] { $x }; foo 1")]
#[case::flag("def foo [--x: any = null] { $x }; foo --x 1")]
fn def_default_value_shouldnt_restrict_explicit_type(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(1)
}

#[rstest]
#[case::positional("def foo [x = 3] { $x }; foo 3.0")]
#[case::flag("def foo2 [--x = 3] { $x }; foo2 --x 3.0")]
fn def_default_value_should_restrict_implicit_type(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::parse_mismatch")
}

#[test]
fn def_wrapped_with_block() -> Result {
    test()
        .run("def --wrapped foo [...rest] { $rest | str join ',' }; foo --bar baz -- -q -u -x")
        .expect_value_eq("--bar,baz,--,-q,-u,-x")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn def_wrapped_from_module() -> Result {
    let code = "
        module spam {
            export def --wrapped my-echo [...rest] { cococo -- ...$rest }
        }

        use spam
        spam my-echo foo -b -as -9 --abc -- -Dxmy=AKOO - bar
    ";

    test()
        .run(code)
        .expect_value_eq("foo -b -as -9 --abc -- -Dxmy=AKOO - bar")
}

#[rstest]
#[case::def_before_signature("def spam --env [] { $env.SPAM = 'spam' }; spam; $env.SPAM")]
#[case::def_before_signature_with_input_output(
    "def spam --env []: nothing -> nothing { $env.SPAM = 'spam' }; spam; $env.SPAM"
)]
#[case::export_def_before_signature(
    "export def spam --env [] { $env.SPAM = 'spam' }; spam; $env.SPAM"
)]
#[case::export_def_before_signature_with_input_output(
    "export def spam --env []: nothing -> nothing { $env.SPAM = 'spam' }; spam; $env.SPAM"
)]
fn cursed_env_flag_positions(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("spam")
}

#[rstest]
#[case::after_signature("def spam [] --env { $env.SPAM = 'spam' }; spam; $env.SPAM")]
#[case::after_block("def spam [] { $env.SPAM = 'spam' } --env; spam; $env.SPAM")]
#[case::after_block_with_input_output(
    "def spam []: nothing -> nothing { $env.SPAM = 'spam' } --env; spam; $env.SPAM"
)]
#[case::export_after_signature("export def spam [] --env { $env.SPAM = 'spam' }; spam; $env.SPAM")]
#[case::export_after_block("export def spam [] { $env.SPAM = 'spam' } --env; spam; $env.SPAM")]
#[case::export_after_block_with_input_output(
    "export def spam []: nothing -> nothing { $env.SPAM = 'spam' } --env; spam; $env.SPAM"
)]
#[ignore = "TODO: Investigate why it's not working, it might be the signature parsing"]
fn cursed_env_flag_positions_after_signature_or_block(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("spam")
}

#[rstest]
#[case::before_signature("def spam --wrapped [...rest] { $rest.0 }; spam --foo")]
#[case::before_signature_with_input_output(
    "def spam --wrapped [...rest]: nothing -> nothing { $rest.0 }; spam --foo"
)]
fn def_cursed_wrapped_flag_positions(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("--foo")
}

#[rstest]
#[case::after_signature("def spam [...rest] --wrapped { $rest.0 }; spam --foo")]
#[case::after_block("def spam [...rest] { $rest.0 } --wrapped; spam --foo")]
#[case::after_block_with_input_output(
    "def spam [...rest]: nothing -> nothing { $rest.0 } --wrapped; spam --foo"
)]
#[ignore = "TODO: Investigate why it's not working, it might be the signature parsing"]
fn def_cursed_wrapped_flag_positions_after_signature_or_block(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("--foo")
}

#[test]
fn def_wrapped_missing_rest_error() -> Result {
    test()
        .run("def --wrapped spam [] {}")
        .expect_error_code_eq("nu::parser::missing_positional")
}

#[test]
fn def_wrapped_wrong_rest_type_error() -> Result {
    let err = test()
        .run("def --wrapped spam [...eggs: list<string>] { $eggs }")
        .expect_parse_error()?;

    let ParseError::TypeMismatchHelp(_, _, _, help) = err else {
        panic!("expected TypeMismatchHelp parse error");
    };
    assert_contains("of ...eggs to 'string'", help);
    Ok(())
}

#[test]
fn def_env_wrapped() -> Result {
    test()
        .run("def --env --wrapped spam [...eggs: string] { $env.SPAM = $eggs.0 }; spam bacon; $env.SPAM")
        .expect_value_eq("bacon")
}

#[test]
fn def_env_wrapped_no_help() -> Result {
    test()
        .run("def --wrapped foo [...rest] { echo $rest }; foo -h")
        .expect_value_eq(["-h"])
}

#[rstest]
#[case::double_quoted(
    r#"def --wrapped foo [...rest] { $rest.0 }; foo expression="releases/4.x.x""#
)]
#[case::single_quoted("def --wrapped foo [...rest] { $rest.0 }; foo expression='releases/4.x.x'")]
fn def_wrapped_untyped_rest_strips_quoted_equals_value(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_value_eq("expression=releases/4.x.x")
}

#[rstest]
#[case::direct("def --wrapped foo [...rest] { $rest | get 0 }; foo ~")]
#[case::external("def --wrapped foo [...rest] { cococo ...$rest }; foo ~")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn def_wrapped_untyped_rest_expands_tilde(#[case] code: &str) -> Result {
    let expected: String = test().run("'~' | path expand")?;
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::double_quoted_equals(
    r#"def --wrapped foo [...rest: string] { $rest.0 }; foo --base="releases/4.x.x""#,
    r#"--base="releases/4.x.x""#
)]
#[case::tilde("def --wrapped foo [...rest: string] { $rest.0 }; foo ~", "~")]
fn def_wrapped_explicit_string_rest_keeps_literal(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::bare_word(
    "def --wrapped foo [...rest] { $rest.0 | describe }; foo test",
    "string"
)]
#[case::multiple_bare_words(
    "def --wrapped foo [...rest] { $rest.2 | describe }; foo a b c",
    "string"
)]
#[case::glob_pattern("def --wrapped foo [...rest] { $rest.0 | describe }; foo *.rs", "glob")]
fn def_wrapped_untyped_rest_describes_arguments(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn def_wrapped_dynamic_percent_builtin_preserves_no_arg_defaults() -> Result {
    Playground::setup(
        "def_wrapped_dynamic_percent_builtin_preserves_no_arg_defaults",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("probe.txt")]);

            test()
                .cwd(dirs.test())
                .run("export def --wrapped builtin [arg1, ...args] { %($arg1) ...$args }; let direct = (ls | where name =~ 'probe.txt' | length); let wrapped = (builtin ls | where name =~ 'probe.txt' | length); [$direct $wrapped]")
                .expect_value_eq([1, 1])
        },
    )
}

#[rstest]
#[case::def(
    "def bar [] { let x = 1; ($x | foo) }; def foo [] { foo }",
    "
        def recursive [c: int] {
            if ($c == 0) { return }
            if ($c mod 2 > 0) {
                $in | recursive ($c - 1)
            } else {
                recursive ($c - 1)
            }
        }
    "
)]
#[case::export_def(
    "export def bar [] { let x = 1; ($x | foo) }; export def foo [] { foo }",
    "
        export def recursive [c: int] {
            if ($c == 0) { return }
            if ($c mod 2 > 0) {
                $in | recursive ($c - 1)
            } else {
                recursive ($c - 1)
            }
        }
    "
)]
fn recursive_func_should_compile(#[case] first_code: &str, #[case] recursive_code: &str) -> Result {
    let (): () = test().run(first_code)?;
    let (): () = test().run(recursive_code)?;

    Ok(())
}
