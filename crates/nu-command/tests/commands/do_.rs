use nu_test_support::{nu, pipeline};

#[test]
fn capture_errors_works() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {$env.use}
        "#
    ));

    assert!(actual.err.contains("column_not_found"));
}

#[test]
fn capture_errors_works_for_external() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail}
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail} | echo `text`
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_semicolon() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail}; echo `text`
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn do_with_semicolon_break_on_failed_external() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do { nu --not_exist_flag }; `text`
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
#[cfg(not(windows))]
fn ignore_error_with_too_much_stderr_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stderr message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                do -i {sh -c "cat a_large_file.txt 1>&2"} | complete | get stderr
            "#
        ));

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn ignore_error_with_too_much_stdout_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::pipeline;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stdout message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(vec![FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                do -i {sh -c "cat a_large_file.txt"} | complete | get stdout
            "#
        ));

        assert_eq!(actual.out, large_file_body);
    })
}

#[test]
#[cfg(not(windows))]
fn ignore_error_with_both_stdout_stderr_messages_not_hang_nushell() {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with many stdout and stderr messages",
        |dirs, sandbox| {
            let script_body = r#"
        x=$(printf '=%.0s' {1..40960})
        echo $x
        echo $x 1>&2
        "#;
            let mut expect_body = String::new();
            for _ in 0..40960 {
                expect_body.push('=');
            }

            sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);

            // check for stdout
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                do -i {bash test.sh} | complete | get stdout | str trim
            "#
            ));
            assert_eq!(actual.out, expect_body);
            // check for stderr
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                do -i {bash test.sh} | complete | get stderr | str trim
            "#
            ));
            assert_eq!(actual.out, expect_body);
        },
    )
}
