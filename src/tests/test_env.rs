use crate::tests::{fail_test, run_test, TestResult};
use nu_test_support::nu;

#[test]
fn shorthand_env_1() -> TestResult {
    run_test(r#"FOO=BAZ $env.FOO"#, "BAZ")
}

#[test]
fn shorthand_env_2() -> TestResult {
    fail_test(r#"FOO=BAZ FOO=MOO $env.FOO"#, "defined_twice")
}

#[test]
fn shorthand_env_3() -> TestResult {
    run_test(r#"FOO=BAZ BAR=MOO $env.FOO"#, "BAZ")
}

#[test]
fn default_nu_lib_dirs_type() {
    let actual = nu!("$env.NU_LIB_DIRS | describe");
    assert_eq!(actual.out, "list<string>");
}

#[test]
fn default_nu_plugin_dirs_type() {
    let actual = nu!("$env.NU_PLUGIN_DIRS | describe");
    assert_eq!(actual.out, "list<string>");
}
