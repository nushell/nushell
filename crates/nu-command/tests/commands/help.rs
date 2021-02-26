use nu_test_support::{nu, pipeline};

#[test]
fn help_commands_count() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        help commands | count
        "#
    ));

    let output = actual.out;
    let output_int: i32 = output.parse().unwrap();
    let is_positive = output_int.is_positive();
    assert!(is_positive);
}

#[test]
fn help_generate_docs_count() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        help generate_docs | flatten | count
        "#
    ));

    let output = actual.out;
    let output_int: i32 = output.parse().unwrap();
    let is_positive = output_int.is_positive();
    assert!(is_positive);
}
