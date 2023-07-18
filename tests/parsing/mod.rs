use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};
use pretty_assertions::assert_eq;

#[test]
fn source_file_relative_to_file() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu source_file_relative.nu
        ");

    assert_eq!(actual.out, "5");
}

#[test]
fn source_const_file() {
    let actual = nu!(cwd: "tests/parsing/samples",
    "
        const file = 'single_line.nu'
        source $file
    ");

    assert_eq!(actual.out, "5");
}

#[test]
fn run_nu_script_single_line() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu single_line.nu
        ");

    assert_eq!(actual.out, "5");
}

#[test]
fn run_nu_script_multiline_start_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_start_pipe.nu
        ");

    assert_eq!(actual.out, "4");
}

#[test]
fn run_nu_script_multiline_start_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_start_pipe_win.nu
        ");

    assert_eq!(actual.out, "3");
}

#[test]
fn run_nu_script_multiline_end_pipe() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_end_pipe.nu
        ");

    assert_eq!(actual.out, "2");
}

#[test]
fn run_nu_script_multiline_end_pipe_win() {
    let actual = nu!(cwd: "tests/parsing/samples", "
        nu multiline_end_pipe_win.nu
        ");

    assert_eq!(actual.out, "3");
}

#[test]
fn parse_file_relative_to_parsed_file_simple() {
    Playground::setup("relative_files_simple", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .mkdir("lol/lol")
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                "
                    use ../lol_shell.nu

                    $env.LOL = (lol_shell ls)
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                source-env lol/lol/lol.nu;
                $env.LOL
            "
        ));

        assert_eq!(actual.out, "lol");
    })
}

#[ignore]
#[test]
fn parse_file_relative_to_parsed_file() {
    Playground::setup("relative_files", |dirs, sandbox| {
        sandbox
            .mkdir("lol")
            .mkdir("lol/lol")
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol/lol.nu",
                "
                    source-env ../../foo.nu
                    use ../lol_shell.nu
                    overlay use ../../lol/lol_shell.nu

                    $env.LOL = $'($env.FOO) (lol_shell ls) (ls)'
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/lol_shell.nu",
                r#"
                    export def ls [] { "lol" }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'foo'
                ",
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                source-env lol/lol/lol.nu;
                $env.LOL
            "
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
                "
                    source-env foo.nu
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "lol/foo.nu",
                "
                    $env.FOO = 'good'
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                source-env lol/lol.nu;
                $env.FOO
            "
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
                "
                    source-env foo.nu
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                "
                    $env.FOO = 'bad'
                ",
            )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                source-env lol/lol.nu
            "
        ));

        assert!(actual.err.contains("File not found"));
    })
}

#[test]
fn parse_export_env_in_module() {
    let actual = nu!("
            module spam { export-env { } }
        ");

    assert!(actual.err.is_empty());
}

#[test]
fn parse_export_env_missing_block() {
    let actual = nu!("
            module spam { export-env }
        ");

    assert!(actual.err.contains("missing block"));
}

#[test]
fn call_command_with_non_ascii_argument() {
    let actual = nu!("
            def nu-arg [--umlaut(-ö): int] {}
            nu-arg -ö 42
        ");

    assert_eq!(actual.err.len(), 0);
}

#[test]
fn parse_long_duration() {
    let actual = nu!(r#"
            "78.797877879789789sec" | into duration
        "#);

    assert_eq!(actual.out, "1min 18sec 797ms");
}
