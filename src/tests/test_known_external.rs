use crate::tests::{fail_test, run_test, run_test_contains, TestResult};

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
fn known_external_complex_unknown_args() -> TestResult {
    run_test_contains(
        "extern echo []; echo foo -b -as -9 --abc -- -Dxmy=AKOO - bar",
        "foo -b -as -9 --abc -- -Dxmy=AKOO - bar",
    )
}

#[test]
fn known_external_batched_short_flag_arg_disallowed() -> TestResult {
    fail_test(
        "extern echo [-a, -b: int]; echo -ab 10",
        "short flag batches",
    )
}

#[test]
fn known_external_missing_positional() -> TestResult {
    fail_test("extern echo [a]; echo", "missing_positional")
}

#[test]
fn known_external_type_mismatch() -> TestResult {
    fail_test("extern echo [a: int]; echo 1.234", "mismatch")
}

#[test]
fn known_external_missing_flag_param() -> TestResult {
    fail_test(
        "extern echo [--foo: string]; echo --foo",
        "missing_flag_param",
    )
}

#[test]
fn known_external_misc_values() -> TestResult {
    run_test(
        r#"
            let x = 'abc'
            extern echo []
            echo $x [ a b c ]
        "#,
        "abc a b c",
    )
}

/// GitHub issue #7822
#[test]
fn known_external_subcommand_from_module() -> TestResult {
    run_test_contains(
        r#"
            module cargo {
                export extern check []
            };
            use cargo;
            cargo check -h
        "#,
        "cargo check",
    )
}

/// GitHub issue #7822
#[test]
fn known_external_aliased_subcommand_from_module() -> TestResult {
    run_test_contains(
        r#"
            module cargo {
                export extern check []
            };
            use cargo;
            alias cc = cargo check;
            cc -h
        "#,
        "cargo check",
    )
}
