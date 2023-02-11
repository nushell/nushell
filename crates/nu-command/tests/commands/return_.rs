use nu_test_support::{nu, pipeline};

#[test]
fn early_return_if_true() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def foo [x] { if true { return 2 }; $x }; foo 100
        "#
    ));

    assert_eq!(actual.out, r#"2"#);
}

#[test]
fn early_return_if_false() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def foo [x] { if false { return 2 }; $x }; foo 100
        "#
    ));

    assert_eq!(actual.out, r#"100"#);
}

#[test]
fn return_works_in_script_without_def_main() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            nu early_return.nu
        "#
    ));

    assert!(actual.err.is_empty());
}
