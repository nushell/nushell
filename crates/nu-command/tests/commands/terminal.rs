use nu_test_support::{nu, pipeline};

// Inside nu! stdout is piped so it won't be a terminal
#[test]
fn is_terminal_stdout_piped() {
    let actual = nu!(pipeline(
        r#"
            is-terminal --stdout
        "#
    ));

    assert_eq!(actual.out, "false");
}

#[test]
fn is_terminal_two_streams() {
    let actual = nu!(pipeline(
        r#"
            is-terminal --stdin --stderr
        "#
    ));

    assert!(actual.err.contains("Only one stream may be checked"));
}
