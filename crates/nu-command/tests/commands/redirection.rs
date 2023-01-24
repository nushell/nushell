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
        let output = nu!(
            cwd: dirs.test(),
            "cat asdfasdfasdf.txt out+err> a; cat a"
        );

        assert!(output.out.contains("asdfasdfasdf.txt"));
    })
}

#[cfg(windows)]
#[test]
fn redirect_outerr() {
    Playground::setup("redirect_outerr_test", |dirs, _sandbox| {
        let output = nu!(
            cwd: dirs.test(),
            "vol missingdrive out+err> a; (open a | size).bytes >= 16"
        );

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
                    r#"bash test.sh out> out.txt err> err.txt"#
                );
            }
            #[cfg(windows)]
            {
                sandbox.with_files(vec![FileWithContent("test.bat", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    r#"cmd /D /c test.bat out> out.txt err> err.txt"#
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
                    r#"bash test.sh out> out.txt err> err.txt; echo $env.LAST_EXIT_CODE"#
                )
            };
            #[cfg(windows)]
            let output = {
                sandbox.with_files(vec![FileWithContent("test.bat", script_body)]);
                nu!(
                    cwd: dirs.test(),
                    r#"cmd /D /c test.bat out> out.txt err> err.txt; echo $env.LAST_EXIT_CODE"#
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
                r#"bash test.sh out> out.txt | describe"#
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(expect_body));
        },
    )
}
