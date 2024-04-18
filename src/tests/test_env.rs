use crate::tests::{fail_test, run_test, TestResult};

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
