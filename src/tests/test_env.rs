use crate::tests::{run_test, TestResult};

#[test]
fn shorthand_env_1() -> TestResult {
    run_test(r#"FOO=BAZ $env.FOO"#, "BAZ")
}

#[test]
fn shorthand_env_2() -> TestResult {
    run_test(r#"FOO=BAZ FOO=MOO $env.FOO"#, "MOO")
}

#[test]
fn shorthand_env_3() -> TestResult {
    run_test(r#"FOO=BAZ BAR=MOO $env.FOO"#, "BAZ")
}

#[test]
fn convert_non_string_env_var_to_nothing() -> TestResult {
    run_test(
        r#"let-env FOO = true; env | where name == FOO | get raw.0 | describe"#,
        "nothing",
    )
}
