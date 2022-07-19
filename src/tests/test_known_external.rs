use crate::tests::{fail_test, run_test_contains, TestResult};

// cargo version prints a string of the form:
// cargo 1.60.0 (d1fd9fe2c 2022-03-01)

#[test]
fn known_external_runs() -> TestResult {
    run_test_contains(r#"extern "cargo version" []; cargo version"#, "cargo")
}

#[test]
fn known_external_unknown_flag() -> TestResult {
    fail_test(
        r#"extern "cargo version" []; cargo version --no-such-flag"#,
        "Found argument '--no-such-flag' which wasn't expected, or isn't valid in this context",
    )
}

#[test]
fn known_external_not_defined_flag() -> TestResult {
    run_test_contains(
        r#"extern "cargo version" []; cargo version --help"#,
        "Show version information",
    )
}

#[test]
fn known_external_is_custom() -> TestResult {
    run_test_contains(
        r#"extern "cargo version" []; help commands | where is_custom == true | get name"#,
        "cargo version",
    )
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
