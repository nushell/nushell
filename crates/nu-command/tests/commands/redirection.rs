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
            "dir missingapplication err> a; (open a | size).bytes >= 16"
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
            "dir missingapplication out+err> a; (open a | size).bytes >= 16"
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

#[cfg(not(windows))]
#[test]
fn separate_redirection() {
    use nu_test_support::fs::{file_contents, Stub::FileWithContent};
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with both stdout and stderr messages, to different file",
        |dirs, sandbox| {
            let script_body = r#"
        x=$(printf '=%.0s' {1..100})
        echo $x
        echo $x 1>&2
        "#;
            let mut expect_body = String::new();
            for _ in 0..100 {
                expect_body.push('=');
            }

            sandbox.with_files(vec![FileWithContent("test.sh", script_body)]);

            nu!(
                cwd: dirs.test(),
                r#"bash test.sh out> out.txt err> err.txt"#
            );
            // check for stdout redirection file.
            let expected_out_file = dirs.test().join("out.txt");
            let actual = file_contents(expected_out_file);
            assert!(actual.contains(&expect_body));

            // check for stderr redirection file.
            let expected_err_file = dirs.test().join("err.txt");
            let actual = file_contents(expected_err_file);
            assert!(actual.contains(&expect_body));
        },
    )
}
