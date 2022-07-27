use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn run_nu_script_single_line() {
    let actual = nu!(cwd: "tests/parsing/samples", r#"
        nu single_line.nu
        "#);

    assert_eq!(actual.out, "5");
}

#[test]
fn run_nu_script_multiline_start_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", r#"
        nu multiline_start_pipe.nu
        "#);

    assert_eq!(actual.out, "4");
}

#[test]
fn run_nu_script_multiline_start_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", r#"
        nu multiline_start_pipe_win.nu
        "#);

    assert_eq!(actual.out, "3");
}

#[test]
fn run_nu_script_multiline_end_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", r#"
        nu multiline_end_pipe.nu
        "#);

    assert_eq!(actual.out, "2");
}

#[test]
fn run_nu_script_multiline_end_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", r#"
        nu multiline_end_pipe_win.nu
        "#);

    assert_eq!(actual.out, "3");
}

#[test]
fn parse_file_relative_to_parsed_file() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .mkdir("lol/lol")
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                r#"
                    source ../../foo.nu
                    use ../lol_shell.nu
                    overlay add ../../lol/lol_shell.nu

                    $'($env.FOO) (lol_shell ls) (ls)'
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    let-env FOO = 'foo'
                "#,
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                source lol/lol/lol.nu
            "#
        ));

        assert_eq!(actual.out, "foo lol lol");
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_1() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol.nu",
                r#"
                    source foo.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/foo.nu",
                r#"
                    let-env FOO = 'good'
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    let-env FOO = 'bad'
                "#,
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                source lol/lol.nu;
                $env.FOO
            "#
        ));

        assert_eq!(actual.out, "good");
    })
}

#[test]
fn parse_file_relative_to_parsed_file_dont_use_cwd_2() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol.nu",
                r#"
                    source foo.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    let-env FOO = 'bad'
                "#,
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                source lol/lol.nu
            "#
        ));

        assert!(actual.err.contains("File not found"));
    })
}
