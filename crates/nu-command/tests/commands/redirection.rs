use nu_test_support::fs::{Stub::FileWithContent, file_contents};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "asdfasdfasdf.txt"; nu --testbin echo_env_stderr BAZ err> a.txt; open a.txt"#
        );
        assert!(output.out.contains("asdfasdfasdf.txt"));

        // check append mode
        let output = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "asdfasdfasdf.txt"; nu --testbin echo_env_stderr BAZ err>> a.txt; open a.txt"#
        );
        let v: Vec<_> = output.out.match_indices("asdfasdfasdf.txt").collect();
        assert_eq!(v.len(), 2);
    })
}

#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "asdfasdfasdf.txt"; nu --testbin echo_env_stderr BAZ out+err> a.txt; open a.txt"#
        );
        assert!(output.out.contains("asdfasdfasdf.txt"));

        let output = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "asdfasdfasdf.txt"; nu --testbin echo_env_stderr BAZ o+e>> a.txt; open a.txt"#
        );
        let v: Vec<_> = output.out.match_indices("asdfasdfasdf.txt").collect();
        assert_eq!(v.len(), 2);
    })
}

#[test]
fn redirect_out() {
    Playground::setup("redirect_out_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "echo 'hello' out> a; open a"
        );

        assert!(output.out.contains("hello"));

        let output = nu!(
            cwd: dirs.test(),
            "echo 'hello' out>> a; open a"
        );
        assert!(output.out.contains("hellohello"));
    })
}

#[test]
fn two_lines_redirection() {
    Playground::setup("redirections with two lines commands", |dirs, _| {
        nu!(
                cwd: dirs.test(),
                r#"
def foobar [] {
    'hello' out> output1.txt
    'world' out> output2.txt
}
foobar"#);
        let file_out1 = dirs.test().join("output1.txt");
        let actual = file_contents(file_out1);
        assert!(actual.contains("hello"));
        let file_out2 = dirs.test().join("output2.txt");
        let actual = file_contents(file_out2);
        assert!(actual.contains("world"));
    })
}

#[test]
fn separate_redirection() {
    Playground::setup(
        "external with both stdout and stderr messages, to different file",
        |dirs, _| {
            let expect_body = "message";
            nu!(
                cwd: dirs.test(),
                r#"$env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ o> out.txt e> err.txt"#,
            );

            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err.txt");
            let actual = file_contents(expected_err_file);
            assert!(actual.contains(expect_body));

            nu!(
                cwd: dirs.test(),
                r#"$env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ o>> out.txt e>> err.txt"#,
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            let v: Vec<_> = actual.match_indices("message").collect();
            assert_eq!(v.len(), 2);

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err.txt");
            let actual = file_contents(expected_err_file);
            let v: Vec<_> = actual.match_indices("message").collect();
            assert_eq!(v.len(), 2);
        },
    )
}

#[test]
fn same_target_redirection_with_too_much_stderr_not_hang_nushell() {
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stderr message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(&[FileWithContent("a_large_file.txt", &large_file_body)]);

        nu!(
            cwd: dirs.test(), pipeline(
                "
                $env.LARGE = (open --raw a_large_file.txt);
                nu --testbin echo_env_stderr LARGE out+err> another_large_file.txt
                "
            ),
        );

        let expected_file = dirs.test().join("another_large_file.txt");
        let actual = file_contents(expected_file);
        assert_eq!(actual, format!("{large_file_body}\n"));

        // not hangs in append mode either.
        let cloned_body = large_file_body.clone();
        large_file_body.push_str(&format!("\n{cloned_body}"));
        nu!(
            cwd: dirs.test(), pipeline(
                "
                $env.LARGE = (open --raw a_large_file.txt);
                nu --testbin echo_env_stderr LARGE out+err>> another_large_file.txt
                "
            ),
        );
        let expected_file = dirs.test().join("another_large_file.txt");
        let actual = file_contents(expected_file);
        assert_eq!(actual, format!("{large_file_body}\n"));
    })
}

#[test]
fn redirection_keep_exit_codes() {
    Playground::setup("redirection preserves exit code", |dirs, _| {
        let out = nu!(
            cwd: dirs.test(),
            "nu --testbin fail e> a.txt | complete | get exit_code"
        );
        // needs to use contains "1", because it complete will output `Some(RawStream)`.
        assert!(out.out.contains('1'));
    });
}

#[test]
fn redirection_stderr_with_failed_program() {
    Playground::setup("redirection stderr with failed program", |dirs, _| {
        let out = nu!(
            cwd: dirs.test(),
            r#"$env.FOO = "bar"; nu --testbin echo_env_stderr_fail FOO e> file.txt; echo 3"#
        );
        // firstly echo 3 shouldn't run, because previous command runs to failed.
        // second `file.txt` should contain "bar".
        assert!(!out.out.contains('3'));
        let expected_file = dirs.test().join("file.txt");
        let actual = file_contents(expected_file);
        assert_eq!(actual, "bar\n");
    });
}

#[test]
fn redirection_with_non_zero_exit_code_should_stop_from_running() {
    Playground::setup("redirection with non zero exit code", |dirs, _| {
        for redirection in ["o>", "o>>", "e>", "e>>", "o+e>", "o+e>>"] {
            let output = nu!(
                cwd: dirs.test(),
                &format!("nu --testbin fail {redirection} log.txt; echo 3")
            );
            assert!(!output.out.contains('3'));
        }
    });

    Playground::setup("redirection with non zero exit code", |dirs, _| {
        for (out, err) in [("o>", "e>"), ("o>>", "e>"), ("o>", "e>>"), ("o>>", "e>>")] {
            let output = nu!(
                cwd: dirs.test(),
                &format!("nu --testbin fail {out} log.txt {err} err_log.txt; echo 3")
            );
            assert!(!output.out.contains('3'));
        }
    })
}

#[test]
fn redirection_with_pipeline_works() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with stdout message with pipeline should write data",
        |dirs, sandbox| {
            let script_body = r"echo message";
            let expect_body = "message";

            sandbox.with_files(&[FileWithContent("test.sh", script_body)]);

            let actual = nu!(
                cwd: dirs.test(),
                r#"$env.BAZ = "message"; nu --testbin echo_env BAZ out> out.txt | describe; open out.txt"#,
            );
            assert!(actual.out.contains(expect_body));

            // check append mode works
            let actual = nu!(
                cwd: dirs.test(),
                r#"$env.BAZ = "message"; nu --testbin echo_env BAZ out>> out.txt | describe; open out.txt"#,
            );
            let v: Vec<_> = actual.out.match_indices("message").collect();
            assert_eq!(v.len(), 2);
        },
    )
}

#[test]
fn redirect_support_variable() {
    Playground::setup("redirect_out_support_variable", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "let x = 'tmp_file'; echo 'hello' out> $x; open tmp_file"
        );

        assert!(output.out.contains("hello"));

        nu!(
            cwd: dirs.test(),
            "let x = 'tmp_file'; echo 'hello there' out+err> $x; open tmp_file"
        );
        // check for stdout redirection file.
        let expected_out_file = dirs.test().join("tmp_file");
        let actual = file_contents(expected_out_file);
        assert!(actual.contains("hello there"));

        // append mode support variable too.
        let output = nu!(
            cwd: dirs.test(),
            "let x = 'tmp_file'; echo 'hello' out>> $x; open tmp_file"
        );
        let v: Vec<_> = output.out.match_indices("hello").collect();
        assert_eq!(v.len(), 2);

        let output = nu!(
            cwd: dirs.test(),
            "let x = 'tmp_file'; echo 'hello' out+err>> $x; open tmp_file"
        );
        // check for stdout redirection file.
        let v: Vec<_> = output.out.match_indices("hello").collect();
        assert_eq!(v.len(), 3);
    })
}

#[test]
fn separate_redirection_support_variable() {
    Playground::setup(
        "external with both stdout and stderr messages, to different file",
        |dirs, _| {
            let expect_body = "message";
            nu!(
                cwd: dirs.test(),
                r#"
            let o_f = "out2.txt"
            let e_f = "err2.txt"
            $env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ o> $o_f e> $e_f"#,
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out2.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err2.txt");
            let actual = file_contents(expected_err_file);
            assert!(actual.contains(expect_body));

            nu!(
                cwd: dirs.test(),
                r#"
            let o_f = "out2.txt"
            let e_f = "err2.txt"
            $env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ out>> $o_f err>> $e_f"#,
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out2.txt");
            let actual = file_contents(expected_out_file);
            let v: Vec<_> = actual.match_indices("message").collect();
            assert_eq!(v.len(), 2);

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err2.txt");
            let actual = file_contents(expected_err_file);
            let v: Vec<_> = actual.match_indices("message").collect();
            assert_eq!(v.len(), 2);
        },
    )
}

#[test]
fn redirection_should_have_a_target() {
    Playground::setup("redirection_should_have_a_target", |dirs, _| {
        let scripts = [
            "echo asdf o+e>",
            "echo asdf o>",
            "echo asdf e>",
            "echo asdf o> e>",
            "echo asdf o> tmp.txt e>",
            "echo asdf o> e> tmp.txt",
            "echo asdf o> | ignore",
            "echo asdf o>; echo asdf",
        ];
        for code in scripts {
            let actual = nu!(cwd: dirs.test(), code);
            assert!(
                actual.err.contains("expected redirection target",),
                "should be error, code: {code}",
            );
            assert!(
                !dirs.test().join("tmp.txt").exists(),
                "No file should be created on error: {code}",
            );
        }
    });
}

#[test]
fn redirection_with_out_pipe() {
    use nu_test_support::playground::Playground;
    Playground::setup("redirection with out pipes", |dirs, _| {
        // check for stdout
        let actual = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ err> tmp_file | str length"#,
        );

        assert_eq!(actual.out, "7");
        // check for stderr redirection file.
        let expected_out_file = dirs.test().join("tmp_file");
        let actual_len = file_contents(expected_out_file).len();
        assert_eq!(actual_len, 8);
    })
}

#[test]
fn redirection_with_err_pipe() {
    use nu_test_support::playground::Playground;
    Playground::setup("redirection with err pipe", |dirs, _| {
        // check for stdout
        let actual = nu!(
            cwd: dirs.test(),
            r#"$env.BAZ = "message"; nu --testbin echo_env_mixed out-err BAZ BAZ out> tmp_file e>| str length"#,
        );

        assert_eq!(actual.out, "7");
        // check for stdout redirection file.
        let expected_out_file = dirs.test().join("tmp_file");
        let actual_len = file_contents(expected_out_file).len();
        assert_eq!(actual_len, 8);
    })
}

#[test]
fn no_redirection_with_outerr_pipe() {
    Playground::setup("redirection does not accept outerr pipe", |dirs, _| {
        for redirect_type in ["o>", "e>", "o+e>"] {
            let actual = nu!(
                cwd: dirs.test(),
                &format!("echo 3 {redirect_type} a.txt o+e>| str length")
            );
            assert!(actual.err.contains("Multiple redirections provided"));
            assert!(
                !dirs.test().join("a.txt").exists(),
                "No file should be created on error"
            );
        }

        // test for separate redirection
        let actual = nu!(
            cwd: dirs.test(),
            "echo 3 o> a.txt e> b.txt o+e>| str length"
        );
        assert!(actual.err.contains("Multiple redirections provided"));
        assert!(
            !dirs.test().join("a.txt").exists(),
            "No file should be created on error"
        );
        assert!(
            !dirs.test().join("b.txt").exists(),
            "No file should be created on error"
        );
    });
}

#[test]
fn no_duplicate_redirection() {
    Playground::setup("redirection does not accept duplicate", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo 3 o> a.txt o> a.txt"
        );
        assert!(actual.err.contains("Multiple redirections provided"));
        assert!(
            !dirs.test().join("a.txt").exists(),
            "No file should be created on error"
        );
        let actual = nu!(
            cwd: dirs.test(),
            "echo 3 e> a.txt e> a.txt"
        );
        assert!(actual.err.contains("Multiple redirections provided"));
        assert!(
            !dirs.test().join("a.txt").exists(),
            "No file should be created on error"
        );
    });
}

#[rstest::rstest]
#[case("let", "out>")]
#[case("let", "err>")]
#[case("let", "out+err>")]
#[case("mut", "out>")]
#[case("mut", "err>")]
#[case("mut", "out+err>")]
fn file_redirection_in_let_and_mut(#[case] keyword: &str, #[case] redir: &str) {
    Playground::setup("file redirection in let and mut", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("$env.BAZ = 'foo'; {keyword} v = nu --testbin echo_env_mixed out-err BAZ BAZ {redir} result.txt")
        );
        assert!(actual.status.success());
        assert!(file_contents(dirs.test().join("result.txt")).contains("foo"));
    })
}

#[rstest::rstest]
#[case("let", "err>|", "foo3")]
#[case("let", "out+err>|", "7")]
#[case("mut", "err>|", "foo3")]
#[case("mut", "out+err>|", "7")]
fn pipe_redirection_in_let_and_mut(
    #[case] keyword: &str,
    #[case] redir: &str,
    #[case] output: &str,
) {
    let actual = nu!(format!(
        "$env.BAZ = 'foo'; {keyword} v = nu --testbin echo_env_mixed out-err BAZ BAZ {redir} str length; $v"
    ));
    assert_eq!(actual.out, output);
}

#[rstest::rstest]
#[case("o>", "bar")]
#[case("e>", "baz")]
#[case("o+e>", "bar\nbaz")] // in subexpression, the stderr is go to the terminal
fn subexpression_redirection(#[case] redir: &str, #[case] stdout_file_body: &str) {
    Playground::setup("file redirection with subexpression", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("$env.BAR = 'bar'; $env.BAZ = 'baz'; (nu --testbin echo_env_mixed out-err BAR BAZ) {redir} result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(
            file_contents(dirs.test().join("result.txt")).trim(),
            stdout_file_body
        );
    })
}

#[rstest::rstest]
#[case("o>", "bar")]
#[case("e>", "baz")]
#[case("o+e>", "bar\nbaz")]
fn file_redirection_in_if_true(#[case] redir: &str, #[case] stdout_file_body: &str) {
    Playground::setup("file redirection if true block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
          format!("$env.BAR = 'bar'; $env.BAZ = 'baz'; if true {{ nu --testbin echo_env_mixed out-err BAR BAZ }} {redir} result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(
            file_contents(dirs.test().join("result.txt")).trim(),
            stdout_file_body
        );
    })
}

#[rstest::rstest]
#[case(true, "hey")]
#[case(false, "ho")]
fn file_redirection_in_if_else(#[case] cond: bool, #[case] stdout_file_body: &str) {
    Playground::setup("file redirection if-else block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("if {cond} {{ echo 'hey' }} else {{ echo 'ho' }} out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(
            file_contents(dirs.test().join("result.txt")).trim(),
            stdout_file_body
        );
    })
}

#[rstest::rstest]
#[case("o>", "bar")]
#[case("e>", "baz")]
#[case("o+e>", "bar\nbaz")]
fn file_redirection_in_try_catch(#[case] redir: &str, #[case] stdout_file_body: &str) {
    Playground::setup("file redirection try-catch block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("$env.BAR = 'bar'; $env.BAZ = 'baz'; try {{ 1/0 }} catch {{ nu --testbin echo_env_mixed out-err BAR BAZ }} {redir} result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(
            file_contents(dirs.test().join("result.txt")).trim(),
            stdout_file_body
        );
    })
}

#[rstest::rstest]
fn file_redirection_where_closure() {
    Playground::setup("file redirection where closure", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("echo foo bar | where {{|x| $x | str contains 'f'}} out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(file_contents(dirs.test().join("result.txt")).trim(), "foo");
    })
}

#[rstest::rstest]
fn file_redirection_match_block() {
    Playground::setup("file redirection match block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("match 3 {{ 1 => 'foo', 3 => 'bar' }} out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(file_contents(dirs.test().join("result.txt")).trim(), "bar");
    })
}

#[rstest::rstest]
fn file_redirection_pattern_match_block() {
    Playground::setup("file redirection pattern match block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("let foo = {{ name: 'bar' }}; match $foo {{ {{ name: 'bar' }} => 'baz' }} out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(file_contents(dirs.test().join("result.txt")).trim(), "baz");
    })
}

#[rstest::rstest]
fn file_redirection_each_block() {
    Playground::setup("file redirection each block", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("[1 2 3] | each {{ $in + 1 }} out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(
            file_contents(dirs.test().join("result.txt")).trim(),
            "2\n3\n4"
        );
    })
}

#[rstest::rstest]
fn file_redirection_do_block_with_return() {
    Playground::setup("file redirection do block with return", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("do {{|x| return ($x + 1); return $x}} 4 out> result.txt")
        );
        assert!(actual.status.success());
        assert_eq!(file_contents(dirs.test().join("result.txt")).trim(), "5");
    })
}

#[rstest::rstest]
fn file_redirection_while_block() {
    Playground::setup("file redirection on while", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("mut x = 0; while $x < 3 {{ $x = $x + 1; echo $x }} o> result.txt")
        );
        // Why the discrepancy to `for` behaviour?
        assert!(actual.status.success());
        assert_eq!(file_contents(dirs.test().join("result.txt")), "");
    })
}

#[rstest::rstest]
fn file_redirection_not_allowed_on_for() {
    Playground::setup("file redirection disallowed on for", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("for $it in [1 2 3] {{ echo $in + 1 }} out> result.txt")
        );
        assert!(!actual.status.success());
        assert!(
            actual.err.contains("Redirection can not be used with for"),
            "Should fail"
        );
    })
}
