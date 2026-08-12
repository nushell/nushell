mod escaping;

use std::time::Duration;

use pretty_assertions::{assert_eq, assert_matches};
use rstest::rstest;

use nu_protocol::Type;
use nu_test_support::fs::Stub;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn source_file_relative_to_file() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu source_file_relative.nu | complete")?;

    assert_eq!(result.stdout.trim(), "5");
    Ok(())
}

#[test]
#[deps(NU)]
fn source_file_relative_to_config() -> Result {
    let code = "
        (
            nu
            --config tests/parsing/samples/source_file_relative.nu
            --commands ''
        )
        | complete
    ";
    let result: CompleteResult = test().run(code)?;

    assert_eq!(result.stdout.trim(), "5");
    Ok(())
}

#[test]
fn source_const_file() -> Result {
    test()
        .cwd("tests/parsing/samples")
        .run("const file = 'single_line.nu'; source $file")
        .expect_value_eq(5)
}

// Regression test for https://github.com/nushell/nushell/issues/17091
// Bare-word string interpolation with constants should work in `source`
#[test]
fn source_const_in_bareword_interpolation() -> Result {
    Playground::setup("source_const_in_bareword_test", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("test_macos.nu", "'macos'"),
            Stub::FileWithContent("test_linux.nu", "'linux'"),
            Stub::FileWithContent("test_windows.nu", "'windows'"),
        ]);

        test()
            .cwd(dirs.test())
            .run("source test_($nu.os-info.name).nu")
            .expect_value_eq(std::env::consts::OS)
    })
}

// Test edge cases for paths with parentheses
#[test]
fn source_path_with_literal_parens() -> Result {
    Playground::setup("source_literal_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "file(with)parens.nu",
            "'literal parens'",
        )]);

        // Quoted path with literal parentheses should work
        test()
            .cwd(dirs.test())
            .run(r#"source "file(with)parens.nu""#)
            .expect_value_eq("literal parens")
    })
}

#[test]
fn source_path_interpolation_vs_literal() -> Result {
    Playground::setup("source_interp_vs_literal_test", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("file(name).nu", "'literal file'"),
            Stub::FileWithContent("file_macos.nu", "'interpolated file'"),
            Stub::FileWithContent("file_linux.nu", "'interpolated file'"),
            Stub::FileWithContent("file_windows.nu", "'interpolated file'"),
        ]);

        // Quoted path should treat parens as literal
        test()
            .cwd(dirs.test())
            .run(r#"source "file(name).nu""#)
            .expect_value_eq("literal file")?;

        // Bare word with parens containing variable should interpolate
        test()
            .cwd(dirs.test())
            .run("source file_($nu.os-info.name).nu")
            .expect_value_eq("interpolated file")
    })
}

#[test]
fn source_path_with_nested_parens() -> Result {
    Playground::setup("source_nested_parens_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[Stub::FileWithContent(
            &format!("test_{}_nested.nu", os_name),
            "'nested parens'",
        )]);

        // Nested parentheses in interpolation
        test()
            .cwd(dirs.test())
            .run("source test_($nu.os-info | get name)_nested.nu")
            .expect_value_eq("nested parens")
    })
}

#[test]
fn source_path_single_quote_no_interpolation() -> Result {
    Playground::setup("source_single_quote_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "file($nu.os-info.name).nu",
            "'no interpolation'",
        )]);

        // Single quotes should prevent interpolation
        test()
            .cwd(dirs.test())
            .run("source 'file($nu.os-info.name).nu'")
            .expect_value_eq("no interpolation")
    })
}

#[test]
fn source_path_backtick_no_interpolation() -> Result {
    Playground::setup("source_backtick_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "file($nu.os-info.name).nu",
            "'backtick no interp'",
        )]);

        // Backticks should also prevent interpolation
        test()
            .cwd(dirs.test())
            .run("source `file($nu.os-info.name).nu`")
            .expect_value_eq("backtick no interp")
    })
}

#[test]
fn source_path_dollar_interpolation() -> Result {
    Playground::setup("source_dollar_interp_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[Stub::FileWithContent(
            &format!("test_{}.nu", os_name),
            "'dollar interpolation'",
        )]);

        // Dollar prefix should enable interpolation in quotes
        test()
            .cwd(dirs.test())
            .run(r#"source $"test_($nu.os-info.name).nu""#)
            .expect_value_eq("dollar interpolation")
    })
}

#[test]
fn source_path_mixed_parens_and_quotes() -> Result {
    Playground::setup("source_mixed_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("test(1).nu", "'test 1'")]);
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[Stub::FileWithContent(
            &format!("test_{}.nu", os_name),
            "'test interpolated'",
        )]);

        // Literal parentheses in quoted string
        test()
            .cwd(dirs.test())
            .run(r#"source "test(1).nu""#)
            .expect_value_eq("test 1")?;

        // Interpolation in bare word with constant
        test()
            .cwd(dirs.test())
            .run("source test_($nu.os-info.name).nu")
            .expect_value_eq("test interpolated")
    })
}

#[test]
fn source_path_empty_parens() -> Result {
    Playground::setup("source_empty_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("file().nu", "'empty parens'")]);

        // Empty parentheses should be treated as literal when quoted
        test()
            .cwd(dirs.test())
            .run(r#"source "file().nu""#)
            .expect_value_eq("empty parens")
    })
}

#[test]
fn source_path_unbalanced_parens_quoted() -> Result {
    Playground::setup("source_unbalanced_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("file(.nu", "'unbalanced open'"),
            Stub::FileWithContent("file).nu", "'unbalanced close'"),
        ]);

        // Unbalanced parentheses should work when quoted
        test()
            .cwd(dirs.test())
            .run(r#"source "file(.nu""#)
            .expect_value_eq("unbalanced open")?;

        test()
            .cwd(dirs.test())
            .run(r#"source "file).nu""#)
            .expect_value_eq("unbalanced close")
    })
}

#[test]
fn source_path_multiple_interpolations() -> Result {
    Playground::setup("source_multiple_interp_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        sandbox.with_files(&[Stub::FileWithContent(
            &format!("{}_{}.nu", os_name, arch),
            "'multiple interpolations'",
        )]);

        // Multiple interpolations in one path using constants
        test()
            .cwd(dirs.test())
            .run("source ($nu.os-info.name)_($nu.os-info.arch).nu")
            .expect_value_eq("multiple interpolations")
    })
}

#[test]
fn source_path_interpolation_with_spaces() -> Result {
    Playground::setup("source_interp_spaces_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "file with spaces.nu",
            "'spaces in name'",
        )]);

        // Spaces in filename require quotes
        test()
            .cwd(dirs.test())
            .run(r#"const name = "file with spaces"; source $"($name).nu""#)
            .expect_value_eq("spaces in name")
    })
}

#[test]
fn source_path_raw_string_no_interpolation() -> Result {
    Playground::setup("source_raw_string_test", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "file($nu.os-info.name).nu",
            "'raw string'",
        )]);

        // Raw strings should not interpolate
        test()
            .cwd(dirs.test())
            .run("source r#'file($nu.os-info.name).nu'#")
            .expect_value_eq("raw string")
    })
}

#[test]
fn source_circular() -> Result {
    test()
        .cwd("tests/parsing/samples")
        .run("source source_circular_1.nu")
        .expect_error_code_eq("nu::parser::circular_import")
}

#[test]
#[deps(NU)]
fn run_nu_script_single_line() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu -n single_line.nu | complete")?;

    assert_eq!(result.stdout.trim(), "5");
    Ok(())
}

#[test]
#[deps(NU)]
fn run_nu_script_multiline_start_pipe() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu -n multiline_start_pipe.nu | complete")?;

    assert_eq!(result.stdout.trim(), "4");
    Ok(())
}

#[test]
#[deps(NU)]
fn run_nu_script_multiline_start_pipe_win() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu -n multiline_start_pipe_win.nu | complete")?;

    assert_eq!(result.stdout.trim(), "3");
    Ok(())
}

#[test]
#[deps(NU)]
fn run_nu_script_multiline_end_pipe() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu -n multiline_end_pipe.nu | complete")?;

    assert_eq!(result.stdout.trim(), "2");
    Ok(())
}

#[test]
#[deps(NU)]
fn run_nu_script_multiline_end_pipe_win() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu -n multiline_end_pipe_win.nu | complete")?;

    assert_eq!(result.stdout.trim(), "3");
    Ok(())
}

#[test]
fn parse_file_relative_to_parsed_file_simple() -> Result {
    Playground::setup("relative_files_simple", |dirs, sandbox| {
        sandbox.mkdir("lol").mkdir("lol/lol").with_files(&[
            Stub::FileWithContent(
                "lol/lol/lol.nu",
                "use ../lol_shell.nu; $env.LOL = (lol_shell ls)",
            ),
            Stub::FileWithContent("lol/lol_shell.nu", r#"export def ls [] { "lol" }"#),
        ]);

        test()
            .cwd(dirs.test())
            .run("source-env lol/lol/lol.nu; $env.LOL")
            .expect_value_eq("lol")
    })
}

#[test]
#[deps(NU)]
fn predecl_signature_single_inp_out_type() -> Result {
    Playground::setup("predecl_signature_single_inp_out_type", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "spam1.nu",
            "
                def main [] { foo }

                def foo []: nothing -> nothing { print 'foo' }
            ",
        )]);

        let result: CompleteResult = test().cwd(dirs.test()).run("nu spam1.nu | complete")?;
        assert_eq!(result.stdout.trim(), "foo");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn predecl_signature_multiple_inp_out_types() -> Result {
    Playground::setup(
        "predecl_signature_multiple_inp_out_types",
        |dirs, sandbox| {
            sandbox.with_files(&[Stub::FileWithContent(
                "spam2.nu",
                "
                def main [] { foo }

                def foo []: [nothing -> string, string -> string] { 'foo' }
            ",
            )]);

            let result: CompleteResult = test().cwd(dirs.test()).run("nu spam2.nu | complete")?;
            assert_eq!(result.stdout.trim(), "foo");

            Ok(())
        },
    )
}

#[test]
fn parse_file_relative_to_parsed_file() -> Result {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox.mkdir("lol").mkdir("lol/lol").with_files(&[
            Stub::FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                "
                    source-env ../../foo.nu
                    use ../lol_shell.nu
                    overlay use ../../lol/lol_shell.nu

                    $env.LOL = $'($env.FOO) (lol_shell ls) (ls)'
                ",
            ),
            Stub::FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            ),
            Stub::FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'foo'
                ",
            ),
        ]);

        test()
            .cwd(dirs.test())
            .run("source-env lol/lol/lol.nu; $env.LOL")
            .expect_value_eq("foo lol lol")
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_1() -> Result {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[Stub::FileWithContentToBeTrimmed(
                "lol/lol.nu",
                "
                    source-env foo.nu
                ",
            )])
            .with_files(&[Stub::FileWithContentToBeTrimmed(
                "lol/foo.nu",
                "
                    $env.FOO = 'good'
                ",
            )])
            .with_files(&[Stub::FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            )]);

        test()
            .cwd(dirs.test())
            .run("source-env lol/lol.nu; $env.FOO")
            .expect_value_eq("good")
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_2() -> Result {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox.mkdir("lol").with_files(&[
            Stub::FileWithContentToBeTrimmed(
                "lol/lol.nu",
                "
                    source-env foo.nu
             ",
            ),
            Stub::FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            ),
        ]);

        test()
            .cwd(dirs.test())
            .run("source-env lol/lol.nu")
            .expect_error_code_eq("nu::parser::sourced_file_not_found")
    })
}

#[test]
fn parse_export_env_in_module() -> Result {
    test().run("module spam { export-env { } }")
}

#[test]
fn parse_export_env_missing_block() -> Result {
    test()
        .run("module spam { export-env }")
        .expect_error_code_eq("nu::parser::missing_positional")
}

#[test]
fn call_command_with_non_ascii_argument() -> Result {
    test().run("def nu-arg [--umlaut(-ö): int] {}; nu-arg -ö 42")
}

#[test]
fn parse_long_duration() -> Result {
    test()
        .run(r#" "78.797877879789789sec" | into duration"#)
        .expect_value_eq(Duration::from_nanos(78797877879))
}

#[rstest]
#[case("def test [ --a: any = 32 ] {}")]
#[case("def test [ --a: number = 32 ] {}")]
#[case("def test [ --a: number = 32.0 ] {}")]
#[case("def test [ --a: list<any> = [ 1 2 3 ] ] {}")]
#[case("def test [ --a: record<a: int b: string> = { a: 32 b: 'qwe' c: 'wqe' } ] {}")]
#[case("def test [ --a: record<a: any b: any> = { a: 32 b: 'qwe'} ] {}")]
#[case("def test []: int -> int { 1 }")]
#[case("def test []: string -> string { 'qwe' }")]
#[case("def test []: nothing -> nothing { null }")]
#[case("def test []: list<string> -> list<string> { [] }")]
#[case("def test []: record<a: int b: int> -> record<c: int e: int> { {c: 1 e: 1} }")]
#[case("def test []: table<a: int b: int> -> table<c: int e: int> { [ {c: 1 e: 1} ] }")]
#[case("def test []: nothing -> record<c: int e: int> { {c: 1 e: 1} }")]
fn parse_function_signature(#[case] phrase: &str) -> Result {
    test().run(phrase)
}

#[test]
fn parse_function_signature_switch_is_bool() -> Result {
    let err = test()
        .run("def foo [--bar] { let baz: int = $bar }")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::Int, Type::Bool, _));
    Ok(())
}

#[rstest]
#[case("def test [ in ] {}")]
#[case("def test [ in: string ] {}")]
#[case("def test [ nu: int ] {}")]
#[case("def test [ env: record<> ] {}")]
#[case("def test [ --env ] {}")]
#[case("def test [ --nu: int ] {}")]
#[case("def test [ --in (-i): list<any> ] {}")]
#[case("def test [ a: string, b: int, in: table<a: int b: int> ] {}")]
#[case("def test [ env, in, nu ] {}")]
fn parse_function_signature_name_is_builtin_var(#[case] phrase: &str) -> Result {
    test()
        .run(phrase)
        .expect_error_code_eq("nu::parser::name_is_builtin_var")
}

#[rstest]
#[case("let a: int = 1")]
#[case("let a: string = 'qwe'")]
#[case("let a: nothing = null")]
#[case("let a: list<string> = []")]
#[case("let a: record<a: int b: int> = {a: 1 b: 1}")]
#[case("let a: table<a: int b: int> = [[a b]; [1 2] [3 4]]")]
#[case("let a: record<a: record<name: string> b: int> = {a: {name: bob} b: 1}")]
fn parse_let_signature(#[case] phrase: &str) -> Result {
    test().run(phrase)
}

#[test]
fn parse_let_signature_missing_colon() -> Result {
    test()
        .run("let a int = 1")
        .expect_error_code_eq("nu::parser::extra_tokens")
}

#[test]
fn parse_mut_signature_missing_colon() -> Result {
    test()
        .run("mut a record<a: int b: int> = {a: 1 b: 1}")
        .expect_error_code_eq("nu::parser::extra_tokens")
}

#[test]
fn parse_const_signature_missing_colon() -> Result {
    test()
        .run("const a string = 'Hello World\n'")
        .expect_error_code_eq("nu::parser::extra_tokens")
}

/// https://github.com/nushell/nushell/issues/16969
#[rstest]
#[case("0..1", "0..(1..2 | first)")]
#[case::lt("0..<1", "0..<(1..2 | first)")]
#[case::eq("0..=1", "0..=(1..2 | first)")]
#[case::no_end("..1", "..(1..2 | first)")]
#[case("1..5..10", "1..(5)..10")]
#[case("1..5..10", "1..(5..10 | first)..10")]
fn wacky_range_parse(#[case] normal: &str, #[case] wacky: &str) -> Result {
    let mut tester = test();
    let expected: Value = tester.run(normal)?;

    tester.run(wacky).expect_value_eq(expected)
}

// Regression test https://github.com/nushell/nushell/issues/17146
#[test]
fn wacky_range_unmatched_paren() -> Result {
    // Unterminated quote is reported as unclosed delimiter (not unexpected EOF).
    test()
        .run("') ..")
        .expect_error_code_eq("nu::parser::unclosed_delimiter")
}

#[test]
#[deps(NU)]
fn issue_16769_recursive_module_command_variable_in_block() -> Result {
    Playground::setup(
        "issue_16769_recursive_module_command_variable_in_block",
        |dirs, sandbox| {
            sandbox.with_files(&[
                Stub::FileWithContentToBeTrimmed(
                    "b.nu",
                    "
                        export def f [] { each {f} }
                    ",
                ),
                Stub::FileWithContentToBeTrimmed(
                    "a.nu",
                    "
                        use b.nu *
                        let i = [];
                        if true { $i | f }
                    ",
                ),
            ]);

            let result: CompleteResult = test().cwd(dirs.test()).run("nu a.nu | complete")?;
            assert_eq!(result.exit_code, 0);
            assert_eq!(result.stderr, "");

            Ok(())
        },
    )
}

#[test]
#[deps(NU)]
fn issue_16769_recursive_module_command_direct_recursion_closure() -> Result {
    Playground::setup(
        "issue_16769_recursive_module_command_direct_recursion_closure",
        |dirs, sandbox| {
            sandbox.with_files(&[
                Stub::FileWithContentToBeTrimmed(
                    "b.nu",
                    "
                        export def f [] { f }
                    ",
                ),
                Stub::FileWithContentToBeTrimmed(
                    "a.nu",
                    "
                        use b.nu f
                        { $in | f }
                    ",
                ),
            ]);

            let result: CompleteResult = test().cwd(dirs.test()).run("nu a.nu | complete")?;
            assert_eq!(result.exit_code, 0);
            assert_eq!(result.stderr, "");

            Ok(())
        },
    )
}

#[test]
fn issue_16769_recursive_module_command_source_def() -> Result {
    Playground::setup(
        "issue_16769_recursive_module_command_source_def",
        |dirs, sandbox| {
            sandbox.with_files(&[
                Stub::FileWithContentToBeTrimmed(
                    "b.nu",
                    "
                    export def f [] { each {f} }
                ",
                ),
                Stub::FileWithContentToBeTrimmed(
                    "a.nu",
                    "
                    use b.nu f
                    def a [] { $in | f }
                ",
                ),
            ]);

            test()
                .cwd(dirs.test())
                .run("source a.nu; [] | f")
                .expect_value_eq([(); 0])
        },
    )
}

#[test]
fn issue_16209_mutual_recursion_closure_in_variable() -> Result {
    let code = "
        def map [] {
            return {
                first: {|| $in | result second | $in + 3 }
                second: {|| $in + 7 | result third | $in * 3 }
                third: {|| $in }
            }
        }
        def result [condition: string, ...args] {
            do (map | get $condition) ...$args
        }
        map | describe
    ";

    test()
        .run(code)
        .expect_value_eq("record<first: closure, second: closure, third: closure>")
}
