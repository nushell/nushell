use nu_test_support::nu;

#[test]
fn print_config_env_default_to_stdout() {
    let actual = nu!("config env --default");
    assert_eq!(
        actual.out,
        nu_utils::get_default_env().replace(['\n', '\r'], "")
    );
    assert!(actual.err.is_empty());
}
