use rstest::rstest;

use nu_test_support::prelude::*;

#[rstest]
#[case::env_shorthand("FOO=BAZ $env.FOO", "BAZ")]
#[case::env_shorthand_multiple("FOO=BAZ BAR=MOO [$env.FOO, $env.BAR]", ["BAZ", "MOO"])]
fn successful(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case::default_env_nu_lib_dirs_type("$env.NU_LIB_DIRS | describe", "list<string>")]
#[case::default_const_nu_lib_dirs_type("$NU_LIB_DIRS | describe", "list<string>")]
// Previously, this was a list<string>
// While we are transitioning to const NU_PLUGIN_DIRS
// the env version will be empty, and thus a
// list<any>
#[case::default_env_plugin_dirs_type("$env.NU_PLUGIN_DIRS | describe", "list<any>")]
#[case::default_const_plugin_dirs_type("$NU_PLUGIN_DIRS | describe", "list<string>")]
#[test]
#[deps(NU)]
fn needs_nu_processes(#[case] child_code: &str, #[case] expect: impl IntoValue) -> Result {
    let code = "let child_code; nu -n -c $child_code";
    test()
        .run_with_data(code, child_code)
        .expect_value_eq(expect)
}

#[test]
fn defined_twice_error() -> Result {
    test()
        .run("FOO=BAZ FOO=MOO $env.FOO")
        .expect_error_code_eq("nu::shell::column_defined_twice")
}
