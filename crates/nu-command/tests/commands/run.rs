use nu_protocol::ShellError;
use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn run_script_without_main_in_pipeline() {
    Playground::setup("run_script_without_main_in_pipeline", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "up.nu",
            "
                str upcase
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run up.nu"#);
        assert_eq!(actual.out, "HELLO");
        assert!(actual.err.is_empty());
    });
}

#[test]
fn run_script_with_main_implicit_in() {
    Playground::setup("run_script_with_main_implicit_in", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "main_up.nu",
            "
                def main [] {
                    $in | str upcase
                }
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run main_up.nu"#);
        assert_eq!(actual.out, "HELLO");
        assert!(actual.err.is_empty());
    });
}

#[test]
fn run_script_with_main_parameters_and_flags() {
    Playground::setup(
        "run_script_with_main_parameters_and_flags",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "format.nu",
                "
                def main [value: string, --char: string] {
                    $\"($value) ($char)\"
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run format.nu --char "!" "#);
            assert_eq!(actual.out, "hello !");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_with_main_parameters_and_short_flags() {
    Playground::setup(
        "run_script_with_main_parameters_and_short_flags",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "format_short.nu",
                "
                def main [value: string, --char(-c): string] {
                    $\"($value) ($char)\"
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run format_short.nu -c "!" "#);
            assert_eq!(actual.out, "hello !");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_without_main_large_input_in_each() {
    Playground::setup(
        "run_script_without_main_large_input_in_each",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "double.nu",
                "
                $in * 2
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), "1..1000 | each { run double.nu } | math sum");
            assert_eq!(actual.out, "1001000");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_does_not_leak_env_from_script_without_main() -> Result {
    Playground::setup(
        "run_does_not_leak_env_from_script_without_main",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "set_env.nu",
                "
                $env.RUN_LOCAL = 'secret'
                $in
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run(r#""hello" | run set_env.nu"#)
                .expect_value_eq("hello")?;
            match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
                ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
                err => Err(err.into()),
            }
        },
    )
}

#[test]
fn run_does_not_leak_env_from_script_main() -> Result {
    Playground::setup("run_does_not_leak_env_from_script_main", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "set_env_main.nu",
            "
                def main [] {
                    $env.RUN_LOCAL = 'secret'
                    $in
                }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        tester
            .run(r#""hello" | run set_env_main.nu"#)
            .expect_value_eq("hello")?;
        match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
            ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
            err => Err(err.into()),
        }
    })
}

#[test]
fn run_missing_script_reports_error() {
    Playground::setup("run_missing_script_reports_error", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#""hello" | run does_not_exist.nu"#);
        assert!(actual.out.is_empty());
        assert!(actual.err.contains("not found") || actual.err.contains("not_exist"));
    });
}

#[test]
fn run_script_parse_error_reports_error() {
    Playground::setup("run_script_parse_error_reports_error", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "bad.nu",
            "
                def main [ {
                    $in
                }
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run bad.nu"#);
        assert!(actual.out.is_empty());
        assert!(!actual.err.is_empty());
    });
}

#[test]
fn run_script_runtime_error_reports_error() {
    Playground::setup("run_script_runtime_error_reports_error", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "runtime_fail.nu",
            "
                def main [] {
                    error make { msg: 'boom from run' }
                }
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run runtime_fail.nu"#);
        assert!(actual.out.is_empty());
        assert!(actual.err.contains("boom from run"));
    });
}

#[test]
fn run_multiple_scripts_in_pipeline() {
    Playground::setup("run_multiple_scripts_in_pipeline", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed(
                "up.nu",
                "
                    str upcase
                ",
            ),
            FileWithContentToBeTrimmed(
                "len.nu",
                "
                    def main [] {
                        str length
                    }
                ",
            ),
        ]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run up.nu | run len.nu"#);
        assert_eq!(actual.out, "5");
        assert!(actual.err.is_empty());
    });
}

#[test]
fn run_nested_pipeline_with_each() {
    Playground::setup("run_nested_pipeline_with_each", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed(
                "up.nu",
                "
                    str upcase
                ",
            ),
            FileWithContentToBeTrimmed(
                "len.nu",
                "
                    def main [] {
                        str length
                    }
                ",
            ),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "['a', 'bb', 'ccc'] | each { |x| $x | run up.nu | run len.nu } | math sum"
        );
        assert_eq!(actual.out, "6");
        assert!(actual.err.is_empty());
    });
}

#[test]
fn run_does_not_cross_script_main_between_invocations() -> Result {
    Playground::setup(
        "run_does_not_cross_script_main_between_invocations",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContentToBeTrimmed(
                    "run-test1.nu",
                    "
                        str upcase
                    ",
                ),
                FileWithContentToBeTrimmed(
                    "run-test2.nu",
                    "
                        def main [] {
                            str length
                        }
                    ",
                ),
            ]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run(r#""hello" | run run-test1.nu"#)
                .expect_value_eq("HELLO")?;
            tester
                .run(r#""hello" | run run-test2.nu"#)
                .expect_value_eq(5)?;
            tester
                .run(r#""hello" | run run-test1.nu"#)
                .expect_value_eq("HELLO")
        },
    )
}

#[test]
fn run_main_script_can_be_invoked_repeatedly() -> Result {
    Playground::setup(
        "run_main_script_can_be_invoked_repeatedly",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-test2.nu",
                "
                def main [] {
                    str length
                }
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run(r#""hello" | run run-test2.nu"#)
                .expect_value_eq(5)?;
            tester
                .run(r#""hello" | run run-test2.nu"#)
                .expect_value_eq(5)?;
            tester
                .run(r#""hello" | run run-test2.nu"#)
                .expect_value_eq(5)
        },
    )
}
