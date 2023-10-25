use nu_test_support::fs::{file_contents, Stub::FileWithContent};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(not(windows))]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt err> a.txt; cat a.txt"
        );

        assert!(output.out.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_err() {
    Playground::setup("redirect_err_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "vol missingdrive err> a; (open a | size).bytes >= 16"
        );

        assert!(output.out.contains("true"));
    })
}

#[cfg(not(windows))]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt out+err> a"
        );
        let output = nu!(cwd: dirs.test(), "cat a");

        assert!(output.out.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "vol missingdrive out+err> a"
        );
        let output = nu!(cwd: dirs.test(), "(open a | size).bytes >= 16");

        assert!(output.out.contains("true"));
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
        |dirs, sandbox| {
            let script_body = r#"
        echo message
        echo message 1>&2
        "#;
            let expect_body = "message";

            #[cfg(not(windows))]
            {
                sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    "bash test.sh out> out.txt err> err.txt"
                );
            }
            #[cfg(windows)]
            {
                sandbox.with_files(vec![FileWithContent("test.bat", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    "cmd /D /c test.bat out> out.txt err> err.txt"
                );
            }
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err.txt");
            let actual = file_contents(expected_err_file);
            assert!(actual.contains(expect_body));
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
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

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
    })
}

#[test]
fn redirection_keep_exit_codes() {
    Playground::setup(
        "redirection should keep exit code the same",
        |dirs, sandbox| {
            let script_body = r#"exit 10"#;
            #[cfg(not(windows))]
            let output = {
                sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    "bash test.sh out> out.txt err> err.txt; echo $env.LAST_EXIT_CODE"
                )
            };
            #[cfg(windows)]
            let output = {
                sandbox.with_files(vec![FileWithContent("test.bat", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    "cmd /D /c test.bat out> out.txt err> err.txt; echo $env.LAST_EXIT_CODE"
                )
            };
            assert_eq!(output.out, "10")
        },
    )
}

#[cfg(not(windows))]
#[test]
fn redirection_with_pipeline_works() {
    use nu_test_support::fs::{file_contents, Stub::FileWithContent};
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with stdout message with pipeline should write data",
        |dirs, sandbox| {
            let script_body = r"echo message";
            let expect_body = "message";
            sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);

            nu!(
                cwd: dirs.test(),
                "bash test.sh out> out.txt | describe"
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));
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

        let output = nu!(
            cwd: dirs.test(),
            "let x = 'tmp_file'; echo 'hello' out+err> $x; open tmp_file"
        );

        assert!(output.out.contains("hello"));
    })
}

#[test]
fn separate_redirection_support_variable() {
    Playground::setup(
        "external with both stdout and stderr messages, to different file",
        |dirs, sandbox| {
            let script_body = r#"
        echo message
        echo message 1>&2
        "#;
            let expect_body = "message";

            #[cfg(not(windows))]
            {
                sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    r#"let o_f = "out.txt"; let e_f = "err.txt"; bash test.sh out> $o_f err> $e_f"#
                );
            }
            #[cfg(windows)]
            {
                sandbox.with_files(vec![FileWithContent("test.bat", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    r#"let o_f = "out.txt"; let e_f = "err.txt"; cmd /D /c test.bat out> $o_f err> $e_f"#
                );
            }
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err.txt");
            let actual = file_contents(expected_err_file);
            assert!(actual.contains(expect_body));
        },
    )
}

#[test]
fn redirection_should_have_a_target() {
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
        let actual = nu!(code);
        assert!(
            actual.err.contains("expected redirection target",),
            "should be error, code: {}",
            code
        );
    }
}
