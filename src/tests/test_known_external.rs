use crate::tests::{fail_test, run_test_contains, TestResult};

// cargo version prints a string of the form:
// cargo 1.60.0 (d1fd9fe2c 2022-03-01)

#[test]
fn known_external_runs() -> TestResult {
    run_test_contains(r#"extern "cargo version" []; cargo version"#, "cargo")
}

#[test]
fn known_external_unknown_flag() -> TestResult {
    run_test_contains(r#"extern "cargo" []; cargo --version"#, "cargo")
}

/// GitHub issues #5179, #4618
#[test]
fn known_external_alias() -> TestResult {
    run_test_contains(
        r#"extern "cargo version" []; alias cv = cargo version; cv"#,
        "cargo",
    )
}

/// GitHub issues #5179, #4618
#[test]
fn known_external_subcommand_alias() -> TestResult {
    run_test_contains(
        r#"extern "cargo version" []; alias c = cargo; c version"#,
        "cargo",
    )
}

#[test]
fn known_external_fall_through_missing_positional() -> TestResult {
    run_test_contains(r#"extern cargo [req_arg: string]; cargo"#, "cargo")
}

#[test]
fn known_external_fall_through_missing_flag_param() -> TestResult {
    fail_test(
        r#"extern "cargo clean" [--package: string]; cargo clean --package"#,
        "requires a SPEC format value",
    )
}
