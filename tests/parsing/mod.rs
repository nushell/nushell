use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[test]
fn source_file_relative_to_file() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu source_file_relative.nu
        ");

    assert_eq!(actual.out, "5");
}

#[test]
fn source_file_relative_to_config() {
    let actual = nu!("
        nu --config tests/parsing/samples/source_file_relative.nu --commands ''
        ");

    assert_eq!(actual.out, "5");
}

#[test]
fn source_const_file() {
    let actual = nu!(cwd: "tests/parsing/samples",
    "
        const file = 'single_line.nu'
        source $file
    ");

    assert_eq!(actual.out, "5");
}

// Regression test for https://github.com/nushell/nushell/issues/17091
// Bare-word string interpolation with constants should work in `source`
#[test]
fn source_const_in_bareword_interpolation() {
    Playground::setup("source_const_in_bareword_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "test_macos.nu",
            "
                print 'macos'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "test_linux.nu",
            "
                print 'linux'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "test_windows.nu",
            "
                print 'windows'
            ",
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "source test_($nu.os-info.name).nu"
        );

        let os_name = std::env::consts::OS;
        assert_eq!(actual.out, os_name);
    });
}

// Test edge cases for paths with parentheses
#[test]
fn source_path_with_literal_parens() {
    Playground::setup("source_literal_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file(with)parens.nu",
            "
                print 'literal parens'
            ",
        )]);

        // Quoted path with literal parentheses should work
        let actual = nu!(
            cwd: dirs.test(),
            r#"source "file(with)parens.nu""#
        );

        assert_eq!(actual.out, "literal parens");
    });
}

#[test]
fn source_path_interpolation_vs_literal() {
    Playground::setup("source_interp_vs_literal_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file(name).nu",
            "
                print 'literal file'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file_macos.nu",
            "
                print 'interpolated file'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file_linux.nu",
            "
                print 'interpolated file'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file_windows.nu",
            "
                print 'interpolated file'
            ",
        )]);

        // Quoted path should treat parens as literal
        let actual_quoted = nu!(
            cwd: dirs.test(),
            r#"source "file(name).nu""#
        );
        assert_eq!(actual_quoted.out, "literal file");

        // Bare word with parens containing variable should interpolate
        let actual_interp = nu!(
            cwd: dirs.test(),
            "source file_($nu.os-info.name).nu"
        );
        assert_eq!(actual_interp.out, "interpolated file");
    });
}

#[test]
fn source_path_with_nested_parens() {
    Playground::setup("source_nested_parens_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            &format!("test_{}_nested.nu", os_name),
            "
                print 'nested parens'
            ",
        )]);

        // Nested parentheses in interpolation
        let actual = nu!(
            cwd: dirs.test(),
            r#"source test_($nu.os-info | get name)_nested.nu"#
        );

        assert_eq!(actual.out, "nested parens");
    });
}

#[test]
fn source_path_single_quote_no_interpolation() {
    Playground::setup("source_single_quote_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file($nu.os-info.name).nu",
            "
                print 'no interpolation'
            ",
        )]);

        // Single quotes should prevent interpolation
        let actual = nu!(
            cwd: dirs.test(),
            r#"source 'file($nu.os-info.name).nu'"#
        );

        assert_eq!(actual.out, "no interpolation");
    });
}

#[test]
fn source_path_backtick_no_interpolation() {
    Playground::setup("source_backtick_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file($nu.os-info.name).nu",
            "
                print 'backtick no interp'
            ",
        )]);

        // Backticks should also prevent interpolation
        let actual = nu!(
            cwd: dirs.test(),
            r#"source `file($nu.os-info.name).nu`"#
        );

        assert_eq!(actual.out, "backtick no interp");
    });
}

#[test]
fn source_path_dollar_interpolation() {
    Playground::setup("source_dollar_interp_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            &format!("test_{}.nu", os_name),
            "
                print 'dollar interpolation'
            ",
        )]);

        // Dollar prefix should enable interpolation in quotes
        let actual = nu!(
            cwd: dirs.test(),
            r#"source $"test_($nu.os-info.name).nu""#
        );

        assert_eq!(actual.out, "dollar interpolation");
    });
}

#[test]
fn source_path_mixed_parens_and_quotes() {
    Playground::setup("source_mixed_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "test(1).nu",
            "
                print 'test 1'
            ",
        )]);
        let os_name = std::env::consts::OS;
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            &format!("test_{}.nu", os_name),
            "
                print 'test interpolated'
            ",
        )]);

        // Literal parentheses in quoted string
        let actual1 = nu!(
            cwd: dirs.test(),
            r#"source "test(1).nu""#
        );
        assert_eq!(actual1.out, "test 1");

        // Interpolation in bare word with constant
        let actual2 = nu!(
            cwd: dirs.test(),
            r#"source test_($nu.os-info.name).nu"#
        );
        assert_eq!(actual2.out, "test interpolated");
    });
}

#[test]
fn source_path_empty_parens() {
    Playground::setup("source_empty_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file().nu",
            "
                print 'empty parens'
            ",
        )]);

        // Empty parentheses should be treated as literal when quoted
        let actual = nu!(
            cwd: dirs.test(),
            r#"source "file().nu""#
        );
        assert_eq!(actual.out, "empty parens");
    });
}

#[test]
fn source_path_unbalanced_parens_quoted() {
    Playground::setup("source_unbalanced_parens_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file(.nu",
            "
                print 'unbalanced open'
            ",
        )]);
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file).nu",
            "
                print 'unbalanced close'
            ",
        )]);

        // Unbalanced parentheses should work when quoted
        let actual1 = nu!(
            cwd: dirs.test(),
            r#"source "file(.nu""#
        );
        assert_eq!(actual1.out, "unbalanced open");

        let actual2 = nu!(
            cwd: dirs.test(),
            r#"source "file).nu""#
        );
        assert_eq!(actual2.out, "unbalanced close");
    });
}

#[test]
fn source_path_multiple_interpolations() {
    Playground::setup("source_multiple_interp_test", |dirs, sandbox| {
        let os_name = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            &format!("{}_{}.nu", os_name, arch),
            "
                print 'multiple interpolations'
            ",
        )]);

        // Multiple interpolations in one path using constants
        let actual = nu!(
            cwd: dirs.test(),
            r#"source ($nu.os-info.name)_($nu.os-info.arch).nu"#
        );
        assert_eq!(actual.out, "multiple interpolations");
    });
}

#[test]
fn source_path_interpolation_with_spaces() {
    Playground::setup("source_interp_spaces_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file with spaces.nu",
            "
                print 'spaces in name'
            ",
        )]);

        // Spaces in filename require quotes
        let actual = nu!(
            cwd: dirs.test(),
            r#"const name = "file with spaces"; source $"($name).nu""#
        );
        assert_eq!(actual.out, "spaces in name");
    });
}

#[test]
fn source_path_raw_string_no_interpolation() {
    Playground::setup("source_raw_string_test", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "file($nu.os-info.name).nu",
            "
                print 'raw string'
            ",
        )]);

        // Raw strings should not interpolate
        let actual = nu!(
            cwd: dirs.test(),
            r#"source r#'file($nu.os-info.name).nu'#"#
        );

        assert_eq!(actual.out, "raw string");
    });
}

#[test]
fn source_circular() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu source_circular_1.nu
        ");

    assert!(actual.err.contains("nu::parser::circular_import"));
}

#[test]
fn run_nu_script_single_line() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu single_line.nu
        ");

    assert_eq!(actual.out, "5");
}

#[test]
fn run_nu_script_multiline_start_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_start_pipe.nu
        ");

    assert_eq!(actual.out, "4");
}

#[test]
fn run_nu_script_multiline_start_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_start_pipe_win.nu
        ");

    assert_eq!(actual.out, "3");
}

#[test]
fn run_nu_script_multiline_end_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_end_pipe.nu
        ");

    assert_eq!(actual.out, "2");
}

#[test]
fn run_nu_script_multiline_end_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_end_pipe_win.nu
        ");

    assert_eq!(actual.out, "3");
}

#[test]
fn parse_file_relative_to_parsed_file_simple() {
    Playground::setup("relative_files_simple", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .mkdir("lol/lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                "
                    use ../lol_shell.nu

                    $env.LOL = (lol_shell ls)
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )]);

        let actual = nu!(cwd: dirs.test(), "
            source-env lol/lol/lol.nu;
            $env.LOL
        ");

        assert_eq!(actual.out, "lol");
    })
}

#[test]
fn predecl_signature_single_inp_out_type() {
    Playground::setup("predecl_signature_single_inp_out_type", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam1.nu",
            "
                def main [] { foo }

                def foo []: nothing -> nothing { print 'foo' }
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), "nu spam1.nu");

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn predecl_signature_multiple_inp_out_types() {
    Playground::setup(
        "predecl_signature_multiple_inp_out_types",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "spam2.nu",
                "
                def main [] { foo }

                def foo []: [nothing -> string, string -> string] { 'foo' }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), "nu spam2.nu");

            assert_eq!(actual.out, "foo");
        },
    )
}

#[ignore]
#[test]
fn parse_file_relative_to_parsed_file() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .mkdir("lol/lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                "
                    source-env ../../foo.nu
                    use ../lol_shell.nu
                    overlay use ../../lol/lol_shell.nu

                    $env.LOL = $'($env.FOO) (lol_shell ls) (ls)'
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'foo'
                ",
            )]);

        let actual = nu!(cwd: dirs.test(), "
            source-env lol/lol/lol.nu;
            $env.LOL
        ");

        assert_eq!(actual.out, "foo lol lol");
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_1() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol.nu",
                "
                    source-env foo.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/foo.nu",
                "
                    $env.FOO = 'good'
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            )]);

        let actual = nu!(cwd: dirs.test(), "
            source-env lol/lol.nu;
            $env.FOO
        ");

        assert_eq!(actual.out, "good");
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_2() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(&[FileWithContentToBeTrimmed(
                "lol/lol.nu",
                "
                    source-env foo.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            )]);

        let actual = nu!(cwd: dirs.test(), "
            source-env lol/lol.nu
        ");

        assert!(actual.err.contains("File not found"));
    })
}

#[test]
fn parse_export_env_in_module() {
    let actual = nu!("
            module spam { export-env { } }
        ");

    assert!(actual.err.is_empty());
}

#[test]
fn parse_export_env_missing_block() {
    let actual = nu!("
            module spam { export-env }
        ");

    assert!(actual.err.contains("missing block"));
}

#[test]
fn call_command_with_non_ascii_argument() {
    let actual = nu!("
            def nu-arg [--umlaut(-ö): int] {}
            nu-arg -ö 42
        ");

    assert_eq!(actual.err.len(), 0);
}

#[test]
fn parse_long_duration() {
    let actual = nu!(r#"
            "78.797877879789789sec" | into duration
        "#);

    assert_eq!(actual.out, "1min 18sec 797ms 877µs 879ns");
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
fn parse_function_signature(#[case] phrase: &str) {
    let actual = nu!(phrase);
    assert!(actual.err.is_empty());
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
fn parse_function_signature_name_is_builtin_var(#[case] phrase: &str) {
    let actual = nu!(phrase);
    assert!(actual.err.contains("nu::parser::name_is_builtin_var"))
}

#[rstest]
#[case("let a: int = 1")]
#[case("let a: string = 'qwe'")]
#[case("let a: nothing = null")]
#[case("let a: list<string> = []")]
#[case("let a: record<a: int b: int> = {a: 1 b: 1}")]
#[case("let a: table<a: int b: int> = [[a b]; [1 2] [3 4]]")]
#[case("let a: record<a: record<name: string> b: int> = {a: {name: bob} b: 1}")]
fn parse_let_signature(#[case] phrase: &str) {
    let actual = nu!(phrase);
    assert!(actual.err.is_empty());
}

#[test]
fn parse_let_signature_missing_colon() {
    let actual = nu!("let a int = 1");
    assert!(actual.err.contains("nu::parser::extra_tokens"));
}

#[test]
fn parse_mut_signature_missing_colon() {
    let actual = nu!("mut a record<a: int b: int> = {a: 1 b: 1}");
    assert!(actual.err.contains("nu::parser::extra_tokens"));
}

#[test]
fn parse_const_signature_missing_colon() {
    let actual = nu!("const a string = 'Hello World\n'");
    assert!(actual.err.contains("nu::parser::extra_tokens"));
}

/// https://github.com/nushell/nushell/issues/16969
#[test]
fn wacky_range_parse() {
    let actual = nu!(r#"0..(1..2 | first)"#);
    assert!(actual.err.is_empty());
}

#[test]
fn wacky_range_parse_lt() {
    let actual = nu!(r#"0..<(1..2 | first)"#);
    assert!(actual.err.is_empty());
}

#[test]
fn wacky_range_parse_eq() {
    let actual = nu!(r#"0..=(1..2 | first)"#);
    assert!(actual.err.is_empty());
}

#[test]
fn wacky_range_parse_no_end() {
    let actual = nu!(r#"..(1..2 | first)"#);
    assert!(actual.err.is_empty());
}

#[test]
fn wacky_range_parse_regression() {
    let actual = nu!(r#"1..(5)..10"#);
    assert!(actual.err.is_empty());
}

#[test]
fn wacky_range_parse_comb() {
    let actual = nu!(r#"1..(5..10 | first)..10"#);
    assert!(actual.err.is_empty());
}

// Regression test https://github.com/nushell/nushell/issues/17146
#[test]
fn wacky_range_unmatched_paren() {
    let actual = nu!(r#"') .."#);
    assert!(!actual.err.is_empty());
}

#[test]
fn parse_let_pipeline_builtin_var() {
    let actual = nu!("1 | let $nu | let $in | let $it | let $env");
    assert!(actual.err.contains("nu::parser::name_is_builtin_var"))
}
