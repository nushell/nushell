use nu_test_support::{nu, pipeline};

#[test]
fn error_label_works() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        error make {msg:foo label:{text:unseen}}
        "#
    ));

    assert!(actual.err.contains("unseen"));
    assert!(actual.err.contains("╰──"));
}

#[test]
fn no_span_if_unspanned() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        error make -u {msg:foo label:{text:unseen}}
        "#
    ));

    assert!(!actual.err.contains("unseen"));
}
