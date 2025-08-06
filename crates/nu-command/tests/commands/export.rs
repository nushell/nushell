use nu_test_support::nu;

#[test]
fn export_command_help() {
    let actual = nu!("export -h");

    assert!(
        actual
            .out
            .contains("Export definitions or environment variables from a module")
    );
}

#[test]
fn export_command_unexpected() {
    let actual = nu!("export foo");

    assert!(actual.err.contains("unexpected export"));
}

#[test]
fn export_alias_should_not_panic() {
    let actual = nu!("export alias");

    assert!(actual.err.contains("Missing"));
}
