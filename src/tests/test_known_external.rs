use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn known_external_runs() -> TestResult {
    run_test(
        r#"extern "cargo run" [-q, --example: string, ...args]; cargo run -q --example test_hello"#,
        "test-hello",
    )
}

#[test]
fn known_external_unknown_flag() -> TestResult {
    fail_test(
        r#"extern "cargo run" [-q, --example: string, ...args]; cargo run -d"#,
        "command doesn't have flag",
    )
}

#[test]
fn known_external_alias() -> TestResult {
    run_test(
        r#"extern "cargo run" [-q, --example: string, ...args]; alias cr = cargo run; cr -q --example test_hello"#,
        "test-hello",
    )
}

#[test]
fn known_external_subcommand_alias() -> TestResult {
    run_test(
        r#"extern "cargo run" [-q, --example: string, ...args]; alias c = cargo; c run -q --example test_hello"#,
        "test-hello",
    )
}
