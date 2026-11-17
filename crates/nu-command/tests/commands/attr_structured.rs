use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;
use std::fs;

/// `cococo` echoes its arguments, so with `@structured` it prints `1,2,3` followed by any
/// arguments the call passes, and the block turns that into a list.
const STRUCTURED_COCOCO: &str = r#"
    @structured ["1,2,3"] { str trim | split row "," }
    extern cococo []
"#;

#[rstest]
#[case::piped_into_internal("cococo | length", test_value!(3))]
#[case::collected("let x = cococo; $x | length", test_value!(3))]
#[case::subexpression("(cococo).1", test_value!("2"))]
#[case::user_args_after_structured_args("cococo 4 | last", test_value!("3 4"))]
#[case::piped_into_external("cococo | relay | str trim", test_value!("cococo"))]
#[case::do_block_into_external("do { cococo } | relay | str trim", test_value!("cococo"))]
#[case::custom_command_into_external(
    "def f [] { cococo }; f | relay | str trim",
    test_value!("cococo")
)]
#[case::subexpression_into_external("(cococo) | relay | str trim", test_value!("cococo"))]
#[case::run_external("cococo | run-external relay | str trim", test_value!("cococo"))]
#[case::in_variable_of_external("cococo | ^cococo $in | str trim", test_value!("cococo"))]
#[case::extern_with_in_variable_into_external(
    "'x' | cococo $in | relay | str trim",
    test_value!("x")
)]
#[case::caret_runs_as_written("^cococo | str trim", test_value!("cococo"))]
#[case::byte_sink("cococo | complete | get stdout | str trim", test_value!("cococo"))]
#[case::tee_passes_own_output("cococo | tee { ignore } | str trim", test_value!("cococo"))]
#[case::stdout_and_stderr("cococo o+e>| str trim", test_value!("cococo"))]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO, TESTBIN_RELAY)]
fn structured_extern(#[case] call: &str, #[case] expected: Value) -> Result {
    test()
        .run(format!("{STRUCTURED_COCOCO}\n{call}"))
        .expect_value_eq(expected)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn structured_extern_from_module() -> Result {
    let code = r#"
        module tools {
            @structured ["1,2,3"] { str trim | split row "," }
            export extern cococo []
        }
        use tools cococo
        cococo | length
    "#;
    test().run(code).expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn structured_extern_to_file_runs_as_written() -> Result {
    Playground::setup("structured_extern_to_file", |dirs, _sandbox| {
        let code = format!("{STRUCTURED_COCOCO}\ncococo o> out.txt; cococo | save saved.txt");
        let () = test().cwd(dirs.test()).run(code)?;
        assert_eq!(
            fs::read_to_string(dirs.test().join("out.txt"))?.trim(),
            "cococo"
        );
        assert_eq!(
            fs::read_to_string(dirs.test().join("saved.txt"))?.trim(),
            "cococo"
        );
        Ok(())
    })
}

/// A string hook runs a structured extern as written, like a closure hook. The decode block
/// writes a file, so the test can tell whether it ran.
#[test]
#[deps(TESTBIN_COCOCO)]
fn string_hook_runs_as_written() -> Result {
    Playground::setup("structured_string_hook", |dirs, _sandbox| {
        let decoded = dirs.test().join("decoded.txt");
        let mut tester = test().cwd(dirs.test());
        let () = tester.run_with_hooks(
            "
                @structured [] { save -f decoded.txt }
                extern cococo []
                $env.config.hooks.pre_execution = ['cococo']
            ",
        )?;
        let () = tester.run_with_hooks("null")?;
        assert!(!decoded.exists(), "the hook decoded the output");

        // The same extern does decode when its output is used.
        let () = tester.run("let x = cococo")?;
        assert!(decoded.exists());
        Ok(())
    })
}

/// Runs `code` in a fresh `nu` and returns what it printed and its exit code.
fn terminal_run(code: String) -> Result<CompleteResult> {
    test().run_with_data("let code; nu -n -c $code | complete", code)
}

/// Runs `code` in a fresh `nu` and returns what it printed to the terminal.
fn terminal_output(code: String) -> Result<String> {
    Ok(terminal_run(code)?.stdout)
}

/// `fail` exits with the code it's given and prints nothing.
const STRUCTURED_FAIL: &str = "
    @structured [] { str trim }
    extern fail [code?: int]
";

/// A failing call's exit-code error follows `display_errors.exit_code`, as for any external,
/// instead of always showing because the decode wrapped it.
#[rstest]
#[case::hidden_by_default("", false)]
#[case::shown_when_enabled("$env.config.display_errors.exit_code = true", true)]
#[nu_test_support::test]
#[deps(NU, TESTBIN_FAIL)]
fn exit_code_error_follows_display_errors(#[case] config: &str, #[case] shown: bool) -> Result {
    let result = terminal_run(format!("{config}\n{STRUCTURED_FAIL}\nfail 3"))?;
    assert_eq!(result.exit_code, 3);
    assert_eq!(
        result.stderr.contains("non_zero_exit_code"),
        shown,
        "{}",
        result.stderr
    );
    Ok(())
}

/// `nu` stands in for a command that prints to stdout and exits non-zero. The block recovers
/// its output with `complete`.
const RECOVERING_NU: &str = r#"
    @structured [] { complete | if $in.exit_code == 0 { "ok" } else { $in.stdout | str trim } }
    extern nu [...args]
"#;

#[test]
#[deps(NU)]
fn decode_recovers_failing_output_at_terminal() -> Result {
    let stdout = terminal_output(format!("{RECOVERING_NU}\nnu -n -c 'print oops; exit 3'"))?;
    assert_eq!(stdout.trim(), "oops");
    Ok(())
}

#[test]
#[deps(NU)]
fn decode_recovers_failing_output_when_collected() -> Result {
    test()
        .run(format!(
            "{RECOVERING_NU}\nlet x = (nu -n -c 'print oops; exit 3'); $x"
        ))
        .expect_value_eq("oops")
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn structured_extern_decodes_to_terminal() -> Result {
    let stdout = terminal_output(format!("{STRUCTURED_COCOCO}\ncococo"))?;
    // The decoded list is shown as a table, not the raw `1,2,3`.
    assert!(!stdout.contains("1,2,3"), "{stdout}");
    for item in ["1", "2", "3"] {
        assert_contains(item, &stdout);
    }
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn structured_extern_runs_as_written_into_print() -> Result {
    let stdout = terminal_output(format!("{STRUCTURED_COCOCO}\ncococo | print"))?;
    assert_eq!(stdout.trim(), "cococo");
    Ok(())
}

/// A structured extern whose result is discarded prints its own text, as an unstructured
/// extern would, instead of being decoded and then dropped.
#[rstest]
#[case::semicolon("cococo; print done", "cococo\ndone")]
#[case::newline("cococo\nprint done", "cococo\ndone")]
#[case::for_body("for i in 1..2 { cococo }", "cococo\ncococo")]
#[case::custom_command_statement("def f [] { cococo; 'x' }; f", "cococo\nx")]
#[case::custom_command_called_as_statement("def f [] { cococo }; f; print done", "cococo\ndone")]
#[nu_test_support::test]
#[deps(NU, TESTBIN_COCOCO)]
fn structured_extern_statement_runs_as_written(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    let stdout = terminal_output(format!("{STRUCTURED_COCOCO}\n{code}"))?;
    assert_eq!(stdout.trim(), expected);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn structured_extern_decodes_through_custom_command_to_terminal() -> Result {
    let stdout = terminal_output(format!("{STRUCTURED_COCOCO}\ndef f [] {{ cococo }}; f"))?;
    assert!(!stdout.contains("1,2,3"), "{stdout}");
    assert_contains("2", &stdout);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn caret_runs_as_written_to_terminal() -> Result {
    let stdout = terminal_output(format!("{STRUCTURED_COCOCO}\n^cococo"))?;
    assert_eq!(stdout.trim(), "cococo");
    Ok(())
}

const PIPE_ONLY_COCOCO: &str = r#"
    @structured --pipe-only ["1,2,3"] { str trim | split row "," }
    extern cococo []
"#;

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn pipe_only_runs_as_written_to_terminal() -> Result {
    let stdout = terminal_output(format!("{PIPE_ONLY_COCOCO}\ncococo"))?;
    assert_eq!(stdout.trim(), "cococo");
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn pipe_only_decodes_when_piped() -> Result {
    test()
        .run(format!("{PIPE_ONLY_COCOCO}\ncococo | length"))
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn decode_reads_let_variables() -> Result {
    let code = r#"
        let x = 5
        @structured ["1,2,3"] { ignore; $x }
        extern cococo []
        let y = cococo
        $y
    "#;
    test().run(code).expect_value_eq(5)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn last_structured_wins() -> Result {
    let code = r#"
        @structured ["1,2,3"] { ignore; "first" }
        @structured ["1,2,3"] { ignore; "second" }
        extern cococo []
        let y = cococo
        $y
    "#;
    test().run(code).expect_value_eq("second")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn decode_error_points_at_call() -> Result {
    let code = r#"
        @structured ["1,2,3"] { ignore; error make {msg: boom} }
        extern cococo []
        cococo | length
    "#;
    let err = test().run(code).expect_shell_error()?;
    assert_eq!(err.to_string(), "boom");
    assert_matches!(
        err,
        ShellError::EvalBlockWithInput(err) if err.label == "error happened when running this"
    );
    Ok(())
}

#[test]
fn decode_rejects_mut_variables() -> Result {
    let code = r#"
        mut m = 1
        @structured ["1,2,3"] { $m }
        extern cococo []
    "#;
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::CaptureOfMutableVar(_));
    Ok(())
}

#[test]
fn structured_needs_a_list_of_args() -> Result {
    let err = test()
        .run("@structured \"-o json\" { from json }\nextern foo []")
        .expect_parse_error()?;
    assert_contains("list<string>", err.to_string());
    Ok(())
}

#[test]
fn structured_on_def_is_an_error() -> Result {
    let err = test()
        .run("@structured [-o json] { from json }\ndef foo [] {}")
        .expect_parse_error()?;
    assert_contains("only applies to `extern`", err.to_string());
    Ok(())
}
