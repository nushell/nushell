use crate::repl::tests::{fail_test, run_test, TestResult};
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
fn default_nu_lib_dirs_env_type() {
    // Previously, this was a list<string>
    // While we are transitioning to const NU_LIB_DIRS
    // the env version will be empty, and thus a
    // list<any>
    let actual = nu!("$env.NU_LIB_DIRS | describe");
    assert_eq!(actual.out, "list<any>");
}

#[test]
fn default_nu_lib_dirs_type() {
    let actual = nu!("$NU_LIB_DIRS | describe");
    assert_eq!(actual.out, "list<string>");
}

#[test]
fn default_nu_plugin_dirs_env_type() {
    // Previously, this was a list<string>
    // While we are transitioning to const NU_PLUGIN_DIRS
    // the env version will be empty, and thus a
    // list<any>
    let actual = nu!("$env.NU_PLUGIN_DIRS | describe");
    assert_eq!(actual.out, "list<any>");
}

#[test]
fn default_nu_plugin_dirs_type() {
    let actual = nu!("$NU_PLUGIN_DIRS | describe");
    assert_eq!(actual.out, "list<string>");
}
