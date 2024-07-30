use crate::repl::tests::{fail_test, run_test, run_test_contains, TestResult};
use std::process::Command;

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
fn known_external_from_module() -> TestResult {
    run_test_contains(
        r#"module spam {
            export extern echo []
        }

        use spam echo
        echo foo -b -as -9 --abc -- -Dxmy=AKOO - bar
        "#,
        "foo -b -as -9 --abc -- -Dxmy=AKOO - bar",
    )
}

#[test]
fn known_external_short_flag_batch_arg_allowed() -> TestResult {
    run_test_contains("extern echo [-a, -b: int]; echo -ab 10", "-b 10")
}

#[test]
fn known_external_short_flag_batch_arg_disallowed() -> TestResult {
    fail_test(
        "extern echo [-a: int, -b]; echo -ab 10",
        "last flag can take args",
    )
}

#[test]
fn known_external_short_flag_batch_multiple_args() -> TestResult {
    fail_test(
        "extern echo [-a: int, -b: int]; echo -ab 10 20",
        "last flag can take args",
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
            extern echo [...args]
            echo $x ...[ a b c ]
        "#,
        "abc a b c",
    )
}

/// GitHub issue #7822
#[test]
fn known_external_subcommand_from_module() -> TestResult {
    let output = Command::new("cargo").arg("add").arg("-h").output()?;
    run_test(
        r#"
            module cargo {
                export extern add []
            };
            use cargo;
            cargo add -h
        "#,
        String::from_utf8(output.stdout)?.trim(),
    )
}

/// GitHub issue #7822
#[test]
fn known_external_aliased_subcommand_from_module() -> TestResult {
    let output = Command::new("cargo").arg("add").arg("-h").output()?;
    run_test(
        r#"
            module cargo {
                export extern add []
            };
            use cargo;
            alias cc = cargo add;
            cc -h
        "#,
        String::from_utf8(output.stdout)?.trim(),
    )
}

#[test]
fn known_external_arg_expansion() -> TestResult {
    run_test(
        r#"
            extern echo [];
            echo ~/foo
        "#,
        &dirs::home_dir()
            .expect("can't find home dir")
            .join("foo")
            .to_string_lossy(),
    )
}

#[test]
fn known_external_arg_quoted_no_expand() -> TestResult {
    run_test(
        r#"
            extern echo [];
            echo "~/foo"
        "#,
        "~/foo",
    )
}

#[test]
fn known_external_arg_internally_quoted_options() -> TestResult {
    run_test(
        r#"
            extern echo [];
            echo --option="test"
        "#,
        "--option=test",
    )
}
