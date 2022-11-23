use nu_test_support::{nu, pipeline};

#[test]
fn help_commands_length() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        help commands | length
        "#
    ));

    let output = actual.out;
    let output_int: i32 = output.parse().unwrap();
    let is_positive = output_int.is_positive();
    assert!(is_positive);
}

#[test]
fn help_shows_signature() {
    let actual = nu!(cwd: ".", pipeline("help str distance"));
    assert!(actual
        .out
        .contains("<string> | str distance <string> -> <int>"));

    // don't show signature for parser keyword
    let actual = nu!(cwd: ".", pipeline("help alias"));
    assert!(!actual.out.contains("Signatures"));
}
