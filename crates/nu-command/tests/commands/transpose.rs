use nu_test_support::{nu, pipeline};

#[test]
fn row() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [[key value]; [foo 1] [foo 2]] | transpose -r | debug
            "#
    ));

    assert_eq!(actual.out, "{{foo: 1}}");
}

#[test]
fn row_but_last() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [[key value]; [foo 1] [foo 2]] | transpose -r -l | debug
            "#
    ));

    assert_eq!(actual.out, "{{foo: 2}}");
}

#[test]
fn row_but_all() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [[key value]; [foo 1] [foo 2]] | transpose -r -a | debug
            "#
    ));

    assert_eq!(actual.out, "{{foo: [1, 2]}}");
}
