#![expect(non_snake_case, reason = "rstest generated some non-snake-case names")]

use nu_protocol::ParseError;
use nu_test_support::{fs::Stub::FileWithContent, prelude::*};
use pretty_assertions::assert_matches;
use rstest::rstest;
use std::{fmt::Write, fs};

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn redirect_err(playground: Playground) -> Result {
    let _: Value = test()
        .cwd(playground.path())
        .run(r#"$env.BAZ = "asdfasdfasdf.txt"; echo_env_stderr BAZ err> a.txt"#)?;
    assert_contains(
        "asdfasdfasdf.txt",
        fs::read_to_string(playground.path().join("a.txt"))?,
    );

    let _: Value = test()
        .cwd(playground.path())
        .run(r#"$env.BAZ = "asdfasdfasdf.txt"; echo_env_stderr BAZ err>> a.txt"#)?;
    let actual = fs::read_to_string(playground.path().join("a.txt"))?;
    let occurrences: Vec<_> = actual.match_indices("asdfasdfasdf.txt").collect();
    assert_eq!(occurrences.len(), 2);
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn redirect_outerr(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run(r#"$env.BAZ = "asdfasdfasdf.txt"; echo_env_stderr BAZ out+err> a.txt"#)?;
    assert_contains(
        "asdfasdfasdf.txt",
        fs::read_to_string(playground.path().join("a.txt"))?,
    );

    let () = test()
        .cwd(playground.path())
        .run(r#"$env.BAZ = "asdfasdfasdf.txt"; echo_env_stderr BAZ o+e>> a.txt"#)?;
    let actual = fs::read_to_string(playground.path().join("a.txt"))?;
    let occurrences: Vec<_> = actual.match_indices("asdfasdfasdf.txt").collect();
    assert_eq!(occurrences.len(), 2);
    Ok(())
}

#[test]
fn redirect_out(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("echo 'hello' out> a")?;
    assert_contains("hello", fs::read_to_string(playground.path().join("a"))?);

    let () = test().cwd(playground.path()).run("echo 'hello' out>> a")?;
    assert_contains("hellohello", fs::read_to_string(playground.path().join("a"))?);
    Ok(())
}

#[test]
fn two_lines_redirection(playground: Playground) -> Result {
    let code = "
        def foobar [] {
            'hello' out> output1.txt
            'world' out> output2.txt
        }
        foobar
    ";

    let () = test().cwd(playground.path()).run(code)?;
    assert_contains(
        "hello",
        fs::read_to_string(playground.path().join("output1.txt"))?,
    );
    assert_contains(
        "world",
        fs::read_to_string(playground.path().join("output2.txt"))?,
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn separate_redirection(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run(
            r#"$env.BAZ = "message"; echo_env_mixed out-err BAZ BAZ o> out.txt e> err.txt"#,
        )?;

        assert_contains("message", fs::read_to_string(playground.path().join("out.txt"))?);
        assert_contains("message", fs::read_to_string(playground.path().join("err.txt"))?);

        let () = test().cwd(playground.path()).run(
            r#"$env.BAZ = "message"; echo_env_mixed out-err BAZ BAZ o>> out.txt e>> err.txt"#,
        )?;

        let actual = fs::read_to_string(playground.path().join("out.txt"))?;
        let occurrences: Vec<_> = actual.match_indices("message").collect();
        assert_eq!(occurrences.len(), 2);

        let actual = fs::read_to_string(playground.path().join("err.txt"))?;
        let occurrences: Vec<_> = actual.match_indices("message").collect();
        assert_eq!(occurrences.len(), 2);
        Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn same_target_redirection_with_too_much_stderr_not_hang_nushell(playground: Playground) -> Result {
    let mut large_file_body = "a".repeat(81920);
    playground.file("a_large_file.txt", &large_file_body)?;

    let code = "
        $env.LARGE = (open --raw a_large_file.txt)
        echo_env_stderr LARGE out+err> another_large_file.txt
    ";
    let () = test().cwd(playground.path()).run(code)?;

    let actual = fs::read_to_string(playground.path().join("another_large_file.txt"))?;
    assert_eq!(actual, format!("{large_file_body}\n"));

    let cloned_body = large_file_body.clone();
    write!(large_file_body, "\n{cloned_body}").expect("writing to a String is infallible");

    let code = "
        $env.LARGE = (open --raw a_large_file.txt)
        echo_env_stderr LARGE out+err>> another_large_file.txt
    ";
    let () = test().cwd(playground.path()).run(code)?;

    let actual = fs::read_to_string(playground.path().join("another_large_file.txt"))?;
    assert_eq!(actual, format!("{large_file_body}\n"));
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn redirection_keep_exit_codes(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run("fail e> a.txt | complete | get exit_code")
        .expect_value_eq(1)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR_FAIL)]
fn redirection_stderr_with_failed_program(playground: Playground) -> Result {
    let code = r#"
        try {
            $env.FOO = "bar"
            echo_env_stderr_fail FOO e> file.txt
            3
        } catch {
            "stopped"
        }
    "#;

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("stopped")?;
    assert_eq!(fs::read_to_string(dirs.test().join("file.txt"))?, "bar\n");
    Ok(())
}

#[rstest]
#[nu_test_support::test]
#[deps(TESTBIN_FAIL)]
fn redirection_with_non_zero_exit_code_should_stop_from_running(
    #[ignore] playground: Playground,
    #[values("o>", "o>>", "e>", "e>>", "o+e>", "o+e>>")] redirection: &str,
) -> Result {
    let code = format!("try {{ fail {redirection} log.txt; 'ran' }} catch {{ 'stopped' }}");
    test().cwd(playground.path()).run(code).expect_value_eq("stopped")
}

#[rstest]
#[nu_test_support::test]
#[deps(TESTBIN_FAIL)]
#[allow(non_snake_case, reason = "rstest generates something odd")]
fn redirection_with_non_zero_exit_code_should_stop_from_running_2(
    #[ignore] playground: Playground,
    #[values(("o>", "e>"), ("o>>", "e>"), ("o>", "e>>"), ("o>>", "e>>"))] (out, err): (&str, &str),
) -> Result {
    let code =
        format!("try {{ fail {out} log.txt {err} err_log.txt; 'ran' }} catch {{ 'stopped' }}");
    test().cwd(playground.path()).run(code).expect_value_eq("stopped")
}

#[test]
#[deps(TESTBIN_ECHO_ENV)]
fn redirection_with_pipeline_works(playground: Playground) -> Result {
    let _: Value = test()
            .cwd(playground.path())
            .run(r#"$env.BAZ = "message"; echo_env BAZ out> out.txt | describe"#)?;
        assert_contains("message", fs::read_to_string(playground.path().join("out.txt"))?);

        let _: Value = test()
            .cwd(playground.path())
            .run(r#"$env.BAZ = "message"; echo_env BAZ out>> out.txt | describe"#)?;
        let actual = fs::read_to_string(playground.path().join("out.txt"))?;
        let occurrences: Vec<_> = actual.match_indices("message").collect();
        assert_eq!(occurrences.len(), 2);
        Ok(())
}

#[test]
fn redirect_support_variable(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("let x = 'tmp_file'; echo 'hello' out> $x")?;
    assert_contains("hello", fs::read_to_string(playground.path().join("tmp_file"))?);

    let () = test()
        .cwd(playground.path())
        .run("let x = 'tmp_file'; echo 'hello there' out+err> $x")?;
    assert_contains(
        "hello there",
        fs::read_to_string(playground.path().join("tmp_file"))?,
    );

    let () = test()
        .cwd(playground.path())
        .run("let x = 'tmp_file'; echo 'hello' out>> $x")?;
    let actual = fs::read_to_string(playground.path().join("tmp_file"))?;
    let occurrences: Vec<_> = actual.match_indices("hello").collect();
    assert_eq!(occurrences.len(), 2);

    let () = test()
        .cwd(playground.path())
        .run("let x = 'tmp_file'; echo 'hello' out+err>> $x")?;
    let actual = fs::read_to_string(playground.path().join("tmp_file"))?;
    let occurrences: Vec<_> = actual.match_indices("hello").collect();
    assert_eq!(occurrences.len(), 3);
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn separate_redirection_support_variable(playground: Playground) -> Result {
    let code = r#"
            let o_f = "out2.txt"
            let e_f = "err2.txt"
            $env.BAZ = "message"
            echo_env_mixed out-err BAZ BAZ o> $o_f e> $e_f
        "#;
        let () = test().cwd(playground.path()).run(code)?;

        assert_contains("message", fs::read_to_string(playground.path().join("out2.txt"))?);
        assert_contains("message", fs::read_to_string(playground.path().join("err2.txt"))?);

        let code = r#"
            let o_f = "out2.txt"
            let e_f = "err2.txt"
            $env.BAZ = "message"
            echo_env_mixed out-err BAZ BAZ out>> $o_f err>> $e_f
        "#;
        let () = test().cwd(playground.path()).run(code)?;

        let actual = fs::read_to_string(playground.path().join("out2.txt"))?;
        let occurrences: Vec<_> = actual.match_indices("message").collect();
        assert_eq!(occurrences.len(), 2);

        let actual = fs::read_to_string(playground.path().join("err2.txt"))?;
        let occurrences: Vec<_> = actual.match_indices("message").collect();
        assert_eq!(occurrences.len(), 2);
        Ok(())
}

#[rstest]
fn redirection_should_have_a_target(
    #[ignore] playground: Playground,
    #[values(
        "echo asdf o+e>",
        "echo asdf o>",
        "echo asdf e>",
        "echo asdf o> e>",
        "echo asdf o> tmp.txt e>",
        "echo asdf o> e> tmp.txt",
        "echo asdf o> | ignore",
        "echo asdf o>; echo asdf"
    )]
    code: &str,
) -> Result {
    let err = test().cwd(playground.path()).run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::Expected("redirection target", _));
    assert!(
        !dirs.test().join("tmp.txt").exists(),
        "No file should be created on error: {code}",
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn redirection_with_out_pipe(playground: Playground) -> Result {
    let code = r#"
        $env.BAZ = "message"
        echo_env_mixed out-err BAZ BAZ err> tmp_file | str length
    "#;
    test().cwd(playground.path()).run(code).expect_value_eq(7)?;

    let actual_len = fs::read_to_string(playground.path().join("tmp_file"))?.len();
    assert_eq!(actual_len, 8);
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn redirection_with_err_pipe(playground: Playground) -> Result {
    let code = r#"
        $env.BAZ = "message"
        echo_env_mixed out-err BAZ BAZ out> tmp_file e>| str length
    "#;
    test().cwd(playground.path()).run(code).expect_value_eq(7)?;

    let actual_len = fs::read_to_string(playground.path().join("tmp_file"))?.len();
    assert_eq!(actual_len, 8);
    Ok(())
}

#[test]
fn no_redirection_with_outerr_pipe(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run("echo 3 o> a.txt e> b.txt o+e>| str length")
        .expect_error_code_eq("nu::parser::multiple_redirections")?;
    assert!(
        !dirs.test().join("a.txt").exists(),
        "No file should be created on error",
    );
    assert!(
        !dirs.test().join("b.txt").exists(),
        "No file should be created on error",
    );
    Ok(())
}

#[rstest]
fn no_redirection_with_outerr_pipe_separate(
    #[ignore] playground: Playground,
    #[values("o>", "e>", "o+e>")] redirect_type: &str,
) -> Result {
    let code = format!("echo 3 {redirect_type} a.txt o+e>| str length");
    test()
        .cwd(playground.path())
        .run(code)
        .expect_error_code_eq("nu::parser::multiple_redirections")?;
    assert!(
        !dirs.test().join("a.txt").exists(),
        "No file should be created on error",
    );
    Ok(())
}

#[rstest]
fn no_duplicate_redirection(#[ignore] playground: Playground, #[values("o>", "e>", "o+e>")] redirect: &str) -> Result {
    test()
        .cwd(playground.path())
        .run(format!("echo 3 {redirect} a.txt {redirect} a.txt"))
        .expect_error_code_eq("nu::parser::multiple_redirections")?;
    assert!(
        !dirs.test().join("a.txt").exists(),
        "No file should be created on error",
    );
    Ok(())
}

#[rstest]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn file_redirection_in_let_and_mut(
    #[ignore] playground: Playground,
    #[values("let", "mut")] keyword: &str,
    #[values(
        "out> result.txt err> other.txt",
        "out> other.txt err> result.txt",
        "out+err> result.txt"
    )]
    redirections: &str,
) -> Result {
    let code = format!(
        "$env.BAZ = 'foo'; {keyword} v = echo_env_mixed out-err BAZ BAZ {redirections}"
    );
    let _: Value = test().cwd(playground.path()).run(code)?;
    assert_contains("foo", fs::read_to_string(playground.path().join("result.txt"))?);
    Ok(())
}

#[rstest]
#[case::let_err("let", "err>|", 3, "foo\n")]
#[case::mut_err("mut", "err>|", 3, "foo\n")]
#[case::let_out_err("let", "out+err>|", 7, None)]
#[case::mut_out_err("mut", "out+err>|", 7, None)]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn pipe_redirection_in_let_and_mut(
    #[ignore] playground: Playground,
    #[case] keyword: &str,
    #[case] redir: &str,
    #[case] output: i64,
    #[case] stdout_file_body: impl Into<Option<&'static str>>,
) -> Result {
    let stdout_file_body = stdout_file_body.into();
    let code = match stdout_file_body {
        Some(_) => format!(
            "$env.BAZ = 'foo'; {keyword} v = echo_env_mixed out-err BAZ BAZ out> stdout.txt {redir} str length; $v"
        ),
        None => format!(
            "$env.BAZ = 'foo'; {keyword} v = echo_env_mixed out-err BAZ BAZ {redir} str length; $v"
        ),
    };

    test().cwd(playground.path()).run(code).expect_value_eq(output)?;
    if let Some(expected) = stdout_file_body {
        assert_eq!(
            fs::read_to_string(dirs.test().join("stdout.txt"))?,
            expected
        );
    }
    Ok(())
}

#[rstest]
#[case::o("o>", "bar")]
#[case::e("e>", "baz")]
#[case::o_e("o+e>", "bar\nbaz")]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn subexpression_redirection(#[ignore] playground: Playground, #[case] redir: &str, #[case] stdout_file_body: &str) -> Result {
    let body = match redir {
        "o>" => "echo_env_mixed out-err BAR BAZ err> other.txt",
        "e>" => "echo_env_mixed out-err BAR BAZ out> other.txt",
        "o+e>" => "echo_env_mixed out-err BAR BAZ",
        _ => unreachable!("unexpected redirection: {redir}"),
    };
    let code = format!("$env.BAR = 'bar'; $env.BAZ = 'baz'; ({body}) {redir} result.txt");
    let _: Value = test().cwd(playground.path()).run(code)?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        stdout_file_body,
    );
    Ok(())
}

#[rstest]
#[case::o("o>", "bar")]
#[case::e("e>", "baz")]
#[case::o_e("o+e>", "bar\nbaz")]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn file_redirection_in_if_true(#[ignore] playground: Playground, #[case] redir: &str, #[case] stdout_file_body: &str) -> Result {
    let body = match redir {
        "o>" => "echo_env_mixed out-err BAR BAZ err> other.txt",
        "e>" => "echo_env_mixed out-err BAR BAZ out> other.txt",
        "o+e>" => "echo_env_mixed out-err BAR BAZ",
        _ => unreachable!("unexpected redirection: {redir}"),
    };
    let code =
        format!("$env.BAR = 'bar'; $env.BAZ = 'baz'; if true {{ {body} }} {redir} result.txt");
    let _: Value = test().cwd(playground.path()).run(code)?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        stdout_file_body,
    );
    Ok(())
}

#[rstest]
#[case::hey(true, "hey")]
#[case::ho(false, "ho")]
fn file_redirection_in_if_else(#[ignore] playground: Playground, #[case] cond: bool, #[case] stdout_file_body: &str) -> Result {
    let code = format!("if {cond} {{ echo 'hey' }} else {{ echo 'ho' }} out> result.txt");
    let () = test().cwd(playground.path()).run(code)?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        stdout_file_body,
    );
    Ok(())
}

#[rstest]
#[case::o("o>", "bar")]
#[case::e("e>", "baz")]
#[case::o_e("o+e>", "bar\nbaz")]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn file_redirection_in_try_catch(#[ignore] playground: Playground, #[case] redir: &str, #[case] stdout_file_body: &str) -> Result {
    let body = match redir {
        "o>" => "echo_env_mixed out-err BAR BAZ err> other.txt",
        "e>" => "echo_env_mixed out-err BAR BAZ out> other.txt",
        "o+e>" => "echo_env_mixed out-err BAR BAZ",
        _ => unreachable!("unexpected redirection: {redir}"),
    };
    let code = format!(
        "$env.BAR = 'bar'; $env.BAZ = 'baz'; try {{ 1/0 }} catch {{ {body} }} {redir} result.txt"
    );
    let _: Value = test().cwd(playground.path()).run(code)?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        stdout_file_body,
    );
    Ok(())
}

#[test]
fn file_redirection_where_closure(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("echo foo bar | where {|x| $x | str contains 'f'} out> result.txt")?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        "foo",
    );
    Ok(())
}

#[test]
fn file_redirection_match_block(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("match 3 { 1 => 'foo', 3 => 'bar' } out> result.txt")?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        "bar",
    );
    Ok(())
}

#[test]
fn file_redirection_pattern_match_block(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run(
        "let foo = { name: 'bar' }; match $foo { { name: 'bar' } => 'baz' } out> result.txt",
    )?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        "baz",
    );
    Ok(())
}

#[test]
fn file_redirection_each_block(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("[1 2 3] | each { $in + 1 } out> result.txt")?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        "2\n3\n4",
    );
    Ok(())
}

#[test]
fn file_redirection_do_block_with_return(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("do {|x| return ($x + 1); return $x} 4 out> result.txt")?;
    assert_eq!(
        fs::read_to_string(dirs.test().join("result.txt"))?.trim(),
        "5",
    );
    Ok(())
}

#[test]
fn file_redirection_while_block(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mut x = 0; while $x < 3 { $x = $x + 1; echo $x } o> result.txt")?;
    assert_eq!(fs::read_to_string(dirs.test().join("result.txt"))?, "");
    Ok(())
}

#[test]
fn file_redirection_not_allowed_on_for(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("for $it in [1 2 3] { echo $in + 1 } out> result.txt")
        .expect_parse_error()?;
    assert_contains("Redirection can not be used with for", err.to_string());
    assert!(
        !dirs.test().join("result.txt").exists(),
        "No file should be created on error",
    );
    Ok(())
}

