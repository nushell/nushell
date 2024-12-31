use nu_test_support::nu;

#[test]
fn force_save_to_dir() {
    let actual = nu!(cwd: "tests/commands", r#"
        "aaa" | save -f ..
        "#);

    assert!(actual.err.contains("Is a directory"));
}
