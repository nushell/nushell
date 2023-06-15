use nu_test_support::nu;

#[test]
fn export_subcommands_help() {
    let actual = nu!("export def -h");

    assert!(actual
        .out
        .contains("Define a custom command and export it from a module"));
}
