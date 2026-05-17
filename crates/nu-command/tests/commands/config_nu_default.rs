use nu_test_support::prelude::*;

#[test]
fn print_config_nu_default_to_stdout() -> Result {
    test()
        .run("config nu --default")
        .expect_value_eq(nu_utils::ConfigFileKind::Config.default())
}
