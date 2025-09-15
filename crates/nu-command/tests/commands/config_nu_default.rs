use nu_test_support::nu;

#[test]
fn print_config_nu_default_to_stdout() {
    let actual = nu!("config nu --default");
    assert_eq!(
        actual.out,
        nu_utils::ConfigFileKind::Config
            .default()
            .replace(['\n', '\r'], "")
    );
    assert!(actual.err.is_empty());
}
