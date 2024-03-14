use nu_test_support::nu;

#[test]
fn basic_stdout() {
    let without_complete = nu!(r#"
        nu --testbin cococo test
    "#);
    let with_complete = nu!(r#"
        (nu --testbin cococo test | complete).stdout
    "#);

    assert_eq!(with_complete.out, without_complete.out);
}

#[test]
fn basic_exit_code() {
    let with_complete = nu!(r#"
        (nu --testbin cococo test | complete).exit_code
    "#);

    assert_eq!(with_complete.out, "0");
}

#[test]
fn error() {
    let actual = nu!("not-found | complete");
    assert!(actual.err.contains("executable was not found"));
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stderr_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stderr message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual =
            nu!(cwd: dirs.test(), "sh -c 'cat a_large_file.txt 1>&2' | complete | get stderr");

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stdout_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stdout message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual = nu!(cwd: dirs.test(), "sh -c 'cat a_large_file.txt' | complete | get stdout");

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_both_stdout_stderr_messages_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with many stdout and stderr messages",
        |dirs, sandbox| {
            let script_body = r#"
        x=$(printf '=%.0s' $(seq 40960))
        echo $x
        echo $x 1>&2
        "#;
            let expect_body = "=".repeat(40960);

            sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);

            // check for stdout
            let actual = nu!(cwd: dirs.test(), "sh test.sh | complete | get stdout | str trim");
            assert_eq!(actual.out, expect_body);
            // check for stderr
            let actual = nu!(cwd: dirs.test(), "sh test.sh | complete | get stderr | str trim");
            assert_eq!(actual.out, expect_body);
        },
    )
}
