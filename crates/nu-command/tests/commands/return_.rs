use nu_test_support::{nu, pipeline};

#[test]
fn early_return_if_true() {
    let actual = nu!("def foo [x] { if true { return 2 }; $x }; foo 100");

    assert_eq!(actual.out, r#"2"#);
}

#[test]
fn early_return_if_false() {
    let actual = nu!("def foo [x] { if false { return 2 }; $x }; foo 100");

    assert_eq!(actual.out, r#"100"#);
}

#[test]
fn return_works_in_script_without_def_main() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "nu -n early_return.nu"
    ));

    assert!(actual.err.is_empty());
}

#[test]
fn return_works_in_script_with_def_main() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        pipeline("nu -n early_return_outside_main.nu")
    );
    assert!(actual.err.is_empty());
}
