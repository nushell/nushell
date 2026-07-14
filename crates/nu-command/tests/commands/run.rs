use nu_protocol::{ParseError, ShellError, parser_path::MAX_RUN_SCRIPT_BYTES};
use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};
use std::io::Write;

#[test]
fn run_script_without_main_in_pipeline() {
    Playground::setup("run_script_without_main_in_pipeline", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "up.nu",
            "
                str uppercase
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
                    $in | str uppercase
                }
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), r#""hello" | run main_up.nu"#);
        assert_eq!(actual.out, "HELLO");
        assert!(actual.err.is_empty());
    });
}

#[test]
fn run_null_passes_pipeline_input_through() {
    Playground::setup("run_null_passes_pipeline_input_through", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), r#""hello" | run null"#);
        assert_eq!(actual.out, "hello");
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
                    $\"($in) ($value) ($char)\"
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run format.nu "arg" --char "!" "#);
            assert_eq!(actual.out, "hello arg !");
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
                    $\"($in) ($value) ($char)\"
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run format_short.nu "arg" -c "!" "#);
            assert_eq!(actual.out, "hello arg !");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_with_main_required_positional_does_not_implicitly_bind_pipeline_input() {
    Playground::setup(
        "run_script_with_main_required_positional_does_not_implicitly_bind_pipeline_input",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "needs_arg.nu",
                "
                def main [value: string] {
                    $value
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run needs_arg.nu"#);
            assert!(actual.out.is_empty());
            assert!(!actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_with_main_keeps_pipeline_input_in_in_when_positional_is_provided() {
    Playground::setup(
        "run_script_with_main_keeps_pipeline_input_in_in_when_positional_is_provided",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "in_and_arg.nu",
                "
                def main [file: path] {
                    $\"($in) -> ($file)\"
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""stream" | run in_and_arg.nu "path.txt""#);
            assert_eq!(actual.out, "stream -> path.txt");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_with_exported_main_uses_main_entrypoint() {
    Playground::setup(
        "run_script_with_exported_main_uses_main_entrypoint",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "exported_main.nu",
                "
                export def main [] {
                    $in | str uppercase
                }
            ",
            )]);

            let actual = nu!(cwd: dirs.test(), r#""hello" | run exported_main.nu"#);
            assert_eq!(actual.out, "HELLO");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_with_exported_env_main_uses_main_entrypoint_without_leaking_env() -> Result {
    Playground::setup(
        "run_script_with_exported_env_main_uses_main_entrypoint_without_leaking_env",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "exported_env_main.nu",
                "
                    export def --env main [] {
                        $env.RUN_LOCAL = 'secret'
                        $in | str uppercase
                    }
                ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run(r#""hello" | run exported_env_main.nu"#)
                .expect_value_eq("HELLO")?;
            match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
                ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
                err => Err(err.into()),
            }
        },
    )
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
                    str uppercase
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
                    str uppercase
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
                        str uppercase
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

#[test]
fn run_main_script_tracks_file_edits_in_repl_session() -> Result {
    Playground::setup(
        "run_main_script_tracks_file_edits_in_repl_session",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-edit.nu",
                "
                def main [] {
                    'hello'
                }
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester.run("run run-edit.nu").expect_value_eq("hello")?;
            tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
            tester
                .run("run run-edit.nu")
                .expect_value_eq("hello world")?;
            tester.run::<()>(r#"'def main [] { "hello" }' | save --force run-edit.nu"#)?;
            tester.run("run run-edit.nu").expect_value_eq("hello")
        },
    )
}

#[test]
fn run_main_script_in_reused_closure_keeps_cached_parse_by_default() -> Result {
    Playground::setup(
        "run_main_script_in_reused_closure_keeps_cached_parse_by_default",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-edit.nu",
                "
                def main [] {
                    'hello'
                }
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester.run::<()>("let runner = { run run-edit.nu }")?;
            tester.run("do $runner").expect_value_eq("hello")?;
            tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
            tester.run("do $runner").expect_value_eq("hello")
        },
    )
}

#[test]
fn run_main_script_in_reused_closure_reloads_with_full_reparse() -> Result {
    Playground::setup(
        "run_main_script_in_reused_closure_reloads_with_full_reparse",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-edit.nu",
                "
                def main [] {
                    'hello'
                }
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester.run::<()>("let runner = { run --full-reparse run-edit.nu }")?;
            tester.run("do $runner").expect_value_eq("hello")?;
            tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
            tester.run("do $runner").expect_value_eq("hello world")?;
            tester.run::<()>(r#"'def main [] { "hello again" }' | save --force run-edit.nu"#)?;
            tester.run("do $runner").expect_value_eq("hello again")
        },
    )
}

#[test]
fn run_script_without_main_tracks_file_edits_with_full_reparse() -> Result {
    Playground::setup(
        "run_script_without_main_tracks_file_edits_with_full_reparse",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-no-main.nu",
                "
                str uppercase
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run(r#""hello" | run --full-reparse run-no-main.nu"#)
                .expect_value_eq("HELLO")?;
            tester.run::<()>("'str downcase' | save --force run-no-main.nu")?;
            tester
                .run(r#""HELLO" | run --full-reparse run-no-main.nu"#)
                .expect_value_eq("hello")
        },
    )
}

#[test]
fn run_full_reparse_recovers_after_script_parse_error() -> Result {
    Playground::setup(
        "run_full_reparse_recovers_after_script_parse_error",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "run-edit.nu",
                "
                def main [] {
                    'ok'
                }
            ",
            )]);

            let mut tester = test().cwd(dirs.test());
            tester
                .run("run --full-reparse run-edit.nu")
                .expect_value_eq("ok")?;
            tester.run::<()>("'def main [ {' | save --force run-edit.nu")?;
            let _ = tester
                .run("run --full-reparse run-edit.nu")
                .expect_shell_error()?;
            tester.run::<()>(r#"'def main [] { "ok again" }' | save --force run-edit.nu"#)?;
            tester
                .run("run --full-reparse run-edit.nu")
                .expect_value_eq("ok again")
        },
    )
}

#[test]
fn run_full_reparse_forwards_main_arguments_and_flags() {
    Playground::setup(
        "run_full_reparse_forwards_main_arguments_and_flags",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "format.nu",
                "
                def main [value: string, --char(-c): string] {
                    $\"($in) ($value) ($char)\"
                }
            ",
            )]);

            let actual = nu!(
                cwd: dirs.test(),
                r#""hello" | run --full-reparse format.nu "arg" -c "!" "#
            );
            assert_eq!(actual.out, "hello arg !");
            assert!(actual.err.is_empty());
        },
    );
}

#[test]
fn run_script_exporting_run_does_not_override_builtin_run_in_repl_session() -> Result {
    Playground::setup(
        "run_script_exporting_run_does_not_override_builtin_run_in_repl_session",
        |dirs, sandbox| {
            sandbox.mkdir("toolkit");
            sandbox.with_files(&[
                FileWithContentToBeTrimmed(
                    "toolkit/wrappers.nu",
                    "
                    export def run [--experimental-options: string] {
                        'toolkit run'
                    }
                ",
                ),
                FileWithContentToBeTrimmed(
                    "toolkit/mod.nu",
                    "
                    export use wrappers.nu *

                    export def main [] {
                        'toolkit main'
                    }
                ",
                ),
                FileWithContentToBeTrimmed(
                    "toolkit.nu",
                    "
                    export use toolkit *

                    export def main [] {
                        help toolkit
                        'ok'
                    }
                ",
                ),
            ]);

            let mut tester = test().cwd(dirs.test());
            tester.run("run toolkit.nu").expect_value_eq("ok")?;
            tester.run("run toolkit.nu").expect_value_eq("ok")
        },
    )
}

#[test]
fn run_script_binds_long_flag_by_name_not_declaration_order() -> Result {
    Playground::setup(
        "run_script_binds_long_flag_by_name_not_declaration_order",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "flags.nu",
                "
                def main [--alpha: int, --beta: int, --gamma: int] {
                    $\"($alpha | default 0)/($beta | default 0)/($gamma | default 0)\"
                }
            ",
            )]);

            // `--gamma` must bind to `--gamma` by name. Previously a long flag
            // matched the first declared flag that had no short character
            // (`--alpha`), so the value silently landed in the wrong slot.
            let mut tester = test().cwd(dirs.test());
            tester
                .run("run flags.nu --gamma 3")
                .expect_value_eq("0/0/3")
        },
    )
}

#[test]
fn run_script_binds_switch_by_name_without_shifting_positional() -> Result {
    Playground::setup(
        "run_script_binds_switch_by_name_without_shifting_positional",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "switch.nu",
                "
                def main [word: string, --num: int, --verbose] {
                    $\"word=($word) num=($num | default 0) verbose=($verbose)\"
                }
            ",
            )]);

            // `--verbose` is a switch declared after the value-taking `--num`.
            // It must bind by name; otherwise it matched `--num`, which then
            // swallowed `hello` as its (int) value and left `word` unbound.
            let mut tester = test().cwd(dirs.test());
            tester
                .run("run switch.nu hello --verbose")
                .expect_value_eq("word=hello num=0 verbose=true")
        },
    )
}

/// Oversized paths must not be loaded by `run` (REPL hang / multi-GiB RAM; #18597).
#[test]
fn run_oversized_file_errors_without_loading() -> Result {
    Playground::setup("run_oversized_file_errors_without_loading", |dirs, _| {
        let path = dirs.test().join("huge.bin");
        let file = std::fs::File::create(&path).expect("create huge.bin");
        // Sparse size only — do not write MAX_RUN_SCRIPT_BYTES of data.
        file.set_len(MAX_RUN_SCRIPT_BYTES + 1)
            .expect("set oversized length");

        let err = test()
            .cwd(dirs.test())
            .run("run huge.bin")
            .expect_parse_error()?;
        assert!(
            matches!(
                err,
                ParseError::ScriptFileTooLarge {
                    size,
                    max_size,
                    ..
                } if size == MAX_RUN_SCRIPT_BYTES + 1 && max_size == MAX_RUN_SCRIPT_BYTES
            ),
            "expected ScriptFileTooLarge, got: {err:?}"
        );
        Ok(())
    })
}

/// Binary files must be rejected before the Nu parser runs (#18597).
#[test]
fn run_binary_file_with_nul_errors_without_parsing() -> Result {
    Playground::setup(
        "run_binary_file_with_nul_errors_without_parsing",
        |dirs, _| {
            let path = dirs.test().join("binary.bin");
            let mut file = std::fs::File::create(&path).expect("create binary.bin");
            file.write_all(b"not\0a\0script")
                .expect("write binary content");

            let err = test()
                .cwd(dirs.test())
                .run("run binary.bin")
                .expect_parse_error()?;
            assert!(
                matches!(err, ParseError::ScriptFileNotText { .. }),
                "expected ScriptFileNotText, got: {err:?}"
            );
            Ok(())
        },
    )
}

/// Invalid UTF-8 (no NULs) must also be rejected as non-text for `run`.
#[test]
fn run_invalid_utf8_file_errors_without_parsing() -> Result {
    Playground::setup("run_invalid_utf8_file_errors_without_parsing", |dirs, _| {
        let path = dirs.test().join("bad_utf8.bin");
        // Lone continuation bytes: invalid UTF-8, no NULs.
        std::fs::write(&path, [0x80, 0x81, 0x82, 0x83, 0xFF]).expect("write invalid utf-8");

        let err = test()
            .cwd(dirs.test())
            .run("run bad_utf8.bin")
            .expect_parse_error()?;
        assert!(
            matches!(err, ParseError::ScriptFileNotText { .. }),
            "expected ScriptFileNotText, got: {err:?}"
        );
        Ok(())
    })
}

/// Dense C0 control characters (no NULs, valid UTF-8 bytes) look like binary to `run`.
#[test]
fn run_control_heavy_file_errors_without_parsing() -> Result {
    Playground::setup(
        "run_control_heavy_file_errors_without_parsing",
        |dirs, _| {
            let path = dirs.test().join("controls.bin");
            // Mostly BEL/SOH-style controls; still valid UTF-8 single bytes, no NULs.
            let mut bytes = vec![0x01u8; 100];
            bytes.extend_from_slice(b"\n");
            std::fs::write(&path, bytes).expect("write control-heavy file");

            let err = test()
                .cwd(dirs.test())
                .run("run controls.bin")
                .expect_parse_error()?;
            assert!(
                matches!(err, ParseError::ScriptFileNotText { .. }),
                "expected ScriptFileNotText, got: {err:?}"
            );
            Ok(())
        },
    )
}

/// `--full-reparse` skips parse-time load, so oversized files must still be rejected at runtime.
#[test]
fn run_full_reparse_oversized_file_errors() -> Result {
    Playground::setup("run_full_reparse_oversized_file_errors", |dirs, _| {
        let path = dirs.test().join("huge.bin");
        let file = std::fs::File::create(&path).expect("create huge.bin");
        file.set_len(MAX_RUN_SCRIPT_BYTES + 1)
            .expect("set oversized length");

        let err = test()
            .cwd(dirs.test())
            .run("run --full-reparse huge.bin")
            .expect_shell_error()?;
        let msg = err.to_string();
        assert!(
            msg.contains("too large") || msg.contains("Script file is too large"),
            "expected too-large shell error, got: {msg}"
        );
        Ok(())
    })
}
