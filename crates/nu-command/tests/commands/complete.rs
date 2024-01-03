use nu_test_support::nu;

#[test]
fn basic_stdout() {
    let without_complete = nu!(r#"
        nu --testbin cococo test
    "#);
    let with_complete = nu!(r#"
        (nu --testbin cococo test | complete).stdout
    "#);

    assert_eq!(with_complete.out, without_complete.out);
}

#[test]
fn basic_exit_code() {
    let with_complete = nu!(r#"
        (nu --testbin cococo test | complete).exit_code
    "#);

    assert_eq!(with_complete.out, "0");
}

#[test]
fn error() {
    let actual = nu!("do { not-found } | complete");

    assert!(actual.err.contains("executable was not found"));
}
