use nu_test_support::nu;

#[test]
fn early_return_if_true() {
    let actual = nu!("def foo [x] { if true { return 2 }; $x }; foo 100");

    assert_eq!(actual.out, "2");
}

#[test]
fn early_return_if_false() {
    let actual = nu!("def foo [x] { if false { return 2 }; $x }; foo 100");

    assert_eq!(actual.out, "100");
}

#[test]
fn return_works_in_script_without_def_main() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", "nu early_return.nu"
    );

    assert!(actual.err.is_empty());
}

#[test]
fn return_with_type_annotation() {
    let actual = nu!("def f [x: int]: any -> int { return (2 * $x) }; f 10");

    assert_eq!(actual.out, "20");
}
