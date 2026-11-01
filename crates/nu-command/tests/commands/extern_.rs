use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn strict_extern_rejects_unknown_long_flag() -> Result {
    let err = test()
        .run(r#"extern --strict "git status" [--short(-s)]; git status --shrt"#)
        .expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::UnknownFlag(_, flag, _, help)
            if flag == "shrt" && help == "Did you mean: `--short`?"
    );
    Ok(())
}

#[test]
fn strict_extern_rejects_unknown_short_flag() -> Result {
    let err = test()
        .run(r#"export extern --strict "git status" [--short(-s)]; git status -x"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::UnknownFlag(_, flag, _, _) if flag == "-x");
    Ok(())
}

#[test]
fn strict_extern_from_module() -> Result {
    let code = r#"
        module git { export extern --strict "git status" [--short(-s)] }
        use git "git status"
        git status --shrt
    "#;
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::UnknownFlag(_, flag, _, _) if flag == "shrt");
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn strict_extern_forwards_double_dash() -> Result {
    // `cococo` rejects `--x` as an option, so this only works if `--` reaches it.
    test()
        .run("extern --strict cococo []; cococo -- --x")
        .expect_value_eq("--x")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn strict_extern_accepts_help() -> Result {
    let result: CompleteResult =
        test().run("extern --strict cococo []; cococo --help | complete")?;
    assert_contains("Usage: cococo", result.stdout);
    Ok(())
}

#[test]
fn strict_extern_reports_params_glued_to_name() -> Result {
    let err = test()
        .run("extern --strict foo[--x]")
        .expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::LabeledErrorWithHelp { error, .. }
            if error == "no space between name and parameters"
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn strict_extern_as_module_main() -> Result {
    let code = "module cococo { export extern --strict main [--a] }; use cococo";
    let err = test()
        .run(format!("{code}; cococo --b"))
        .expect_parse_error()?;
    assert_matches!(err, ParseError::UnknownFlag(_, flag, _, _) if flag == "b");
    test()
        .run(format!("{code}; cococo -- --b"))
        .expect_value_eq("--b")
}

#[rstest]
#[case::count("-10", "-1")]
#[case::short_variable("-$n", "-$")]
#[case::short_subexpression("-($n)", "-3")]
#[case::long_subexpression("--($flag)", "--x")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn strict_extern_forwards_counts_and_run_time_flags(
    #[case] arg: &str,
    #[case] option: &str,
) -> Result {
    // cococo rejects every option, so its error names the one that reached it.
    let code = format!(
        r#"extern --strict cococo []; let n = 3; let flag = "x"; cococo {arg} | complete | get stderr"#
    );
    let stderr: String = test().run(code)?;
    assert_contains(format!("'{option}'"), stderr);
    Ok(())
}

#[rstest]
#[case::extern_("extern foo [--x: string]; foo --x (ls --bogus) hello")]
#[case::strict_extern("extern --strict foo [--x: string]; foo --x (ls --bogus) hello")]
#[case::def_wrapped("def --wrapped foo [--x: string, ...rest] {}; foo --x (ls --bogus) hello")]
fn unknown_flag_in_flag_value_is_reported(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::UnknownFlag(command, flag, _, _) if command == "ls" && flag == "bogus"
    );
    Ok(())
}
