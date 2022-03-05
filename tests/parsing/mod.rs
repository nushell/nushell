use nu_test_support::nu;

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
