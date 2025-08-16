use nu_test_support::nu;

#[test]
fn export_subcommands_help() {
    let actual = nu!("export def -h");

    assert!(
        actual
            .out
            .contains("Define a custom command and export it from a module")
    );
}

#[test]
fn export_should_not_expose_arguments() {
    // issue #16211
    let actual = nu!(r#"
            export def foo [bar: int] {}
            scope variables
        "#);

    assert!(!actual.out.contains("bar"));
}
