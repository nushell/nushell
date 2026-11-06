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

#[rstest]
#[case::terminal("cococo")]
#[case::print("cococo | print")]
#[nu_test_support::test]
#[deps(NU, TESTBIN_COCOCO)]
fn structured_extern_runs_as_written_to_terminal(#[case] call: &str) -> Result {
    let result: CompleteResult = test().run_with_data(
        "let code; nu -n -c $code | complete",
        format!("{STRUCTURED_COCOCO}\n{call}"),
    )?;
    assert_eq!(result.stdout.trim(), "cococo");
    Ok(())
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
