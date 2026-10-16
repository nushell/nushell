use nu_protocol::{ParseError, ShellError, parser_path::MAX_RUN_SCRIPT_BYTES};
use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};
use pretty_assertions::assert_matches;
use rstest::rstest;
use std::io::Write;

#[test]
fn run_script_without_main_in_pipeline() -> Result {
    Playground::setup("run_script_without_main_in_pipeline", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "up.nu",
            "
                str uppercase
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run(r#""hello" | run up.nu"#)
            .expect_value_eq("HELLO")
    })
}

#[test]
fn run_script_with_main_implicit_in() -> Result {
    Playground::setup("run_script_with_main_implicit_in", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "main_up.nu",
            "
                def main [] {
                    $in | str uppercase
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run(r#""hello" | run main_up.nu"#)
            .expect_value_eq("HELLO")
    })
}

#[test]
fn run_null_passes_pipeline_input_through() -> Result {
    Playground::setup("run_null_passes_pipeline_input_through", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run(r#""hello" | run null"#)
            .expect_value_eq("hello")
    })
}

#[test]
fn run_script_with_main_parameters_and_flags() -> Result {
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

            test()
                .cwd(dirs.test())
                .run(r#""hello" | run format.nu "arg" --char "!" "#)
                .expect_value_eq("hello arg !")
        },
    )
}

#[test]
fn run_script_with_main_parameters_and_short_flags() -> Result {
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

            test()
                .cwd(dirs.test())
                .run(r#""hello" | run format_short.nu "arg" -c "!" "#)
                .expect_value_eq("hello arg !")
        },
    )
}

#[test]
fn run_script_with_main_required_positional_does_not_implicitly_bind_pipeline_input() -> Result {
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

            let _ = test()
                .cwd(dirs.test())
                .run(r#""hello" | run needs_arg.nu"#)
                .expect_error()?;
            Ok(())
        },
    )
}

#[test]
fn run_script_with_main_keeps_pipeline_input_in_in_when_positional_is_provided() -> Result {
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

            test()
                .cwd(dirs.test())
                .run(r#""stream" | run in_and_arg.nu "path.txt""#)
                .expect_value_eq("stream -> path.txt")
        },
    )
}

#[test]
fn run_script_with_exported_main_uses_main_entrypoint() -> Result {
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

            test()
                .cwd(dirs.test())
                .run(r#""hello" | run exported_main.nu"#)
                .expect_value_eq("HELLO")
        },
    )
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
fn run_script_without_main_large_input_in_each() -> Result {
    Playground::setup(
        "run_script_without_main_large_input_in_each",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "double.nu",
                "
                $in * 2
            ",
            )]);

            test()
                .cwd(dirs.test())
                .run("1..1000 | each { run double.nu } | math sum")
                .expect_value_eq(1001000)
        },
    )
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
fn run_missing_script_reports_error() -> Result {
    Playground::setup("run_missing_script_reports_error", |dirs, _| {
        let err = test()
            .cwd(dirs.test())
            .run(r#""hello" | run does_not_exist.nu"#)
            .expect_parse_error()?;
        assert!(
            matches!(err, ParseError::SourcedFileNotFound(ref path, _) if path == "does_not_exist.nu"),
            "expected missing script error, got: {err:?}"
        );
        Ok(())
    })
}

#[test]
fn run_script_parse_error_reports_error() -> Result {
    Playground::setup("run_script_parse_error_reports_error", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "bad.nu",
            "
                def main [ {
                    $in
                }
            ",
        )]);

        let _ = test()
            .cwd(dirs.test())
            .run(r#""hello" | run bad.nu"#)
            .expect_parse_error()?;
        Ok(())
    })
}

#[test]
fn run_script_runtime_error_reports_error() -> Result {
    Playground::setup("run_script_runtime_error_reports_error", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "runtime_fail.nu",
            "
                def main [] {
                    error make { msg: 'boom from run' }
                }
            ",
        )]);

        let err = test()
            .cwd(dirs.test())
            .run(r#""hello" | run runtime_fail.nu"#)
            .expect_error()?;
        assert_contains("boom from run", err.to_string());
        Ok(())
    })
}

#[test]
fn run_multiple_scripts_in_pipeline() -> Result {
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

        test()
            .cwd(dirs.test())
            .run(r#""hello" | run up.nu | run len.nu"#)
            .expect_value_eq(5)
    })
}

#[test]
fn run_nested_pipeline_with_each() -> Result {
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

        test()
            .cwd(dirs.test())
            .run("['a', 'bb', 'ccc'] | each { |x| $x | run up.nu | run len.nu } | math sum")
            .expect_value_eq(6)
    })
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
fn run_full_reparse_forwards_main_arguments_and_flags() -> Result {
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

            test()
                .cwd(dirs.test())
                .run(r#""hello" | run --full-reparse format.nu "arg" -c "!" "#)
                .expect_value_eq("hello arg !")
        },
    )
}

#[test]
fn run_script_with_toolkit_like_exports_can_be_run_twice_in_repl_session() -> Result {
    // Regression: running a toolkit-style script (with exports) must not break
    // subsequent `run` invocations in the same REPL session. The export must not
    // be named `run` — that is a parser keyword and is rejected.
    Playground::setup(
        "run_script_with_toolkit_like_exports_can_be_run_twice_in_repl_session",
        |dirs, sandbox| {
            sandbox.mkdir("toolkit");
            sandbox.with_files(&[
                FileWithContentToBeTrimmed(
                    "toolkit/wrappers.nu",
                    "
                    export def dev [--experimental-options: string] {
                        'toolkit dev'
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
        assert_contains("too large", err.to_string());
        Ok(())
    })
}

/// The script from issue #19266.
const FLAGS_SCRIPT: &str = "
    def main [--foo(-f): string, --bar(-b)] {
        {foo: $foo, bar: $bar}
    }
";

#[rstest]
#[case::issue_19266(r#"run flags.nu --foo "23" --bar=true"#, Some("23"), true)]
// `-f` after the path is `main`'s `--foo`, not `run --full-reparse`.
#[case::flags_after_the_path("run flags.nu -f x -b", Some("x"), true)]
#[case::full_reparse(r#"run --full-reparse flags.nu -f "23" --bar=true"#, Some("23"), true)]
#[case::caller_variables(
    r#"do {|value| run flags.nu --foo $value } "from closure""#,
    Some("from closure"),
    false
)]
// The path follows `--`, even when it starts with `-`.
#[case::path_after_double_dash("run -- -flags.nu --foo x", Some("x"), false)]
// The parser splits `--name=value`, so the value is evaluated.
#[case::inline_value(r#"let v = "hi"; run flags.nu --foo=$v"#, Some("hi"), false)]
// A flag takes the next argument as its value, as in a call to `main`.
#[case::dash_value("run flags.nu --foo -b", Some("-b"), false)]
// As in a call to `main`, `null` leaves out a flag that doesn't take it.
#[case::null_value("let v = null; run flags.nu --foo $v", None, false)]
// A batch of short flags sets each one, and the last one takes the next argument.
#[case::short_flag_batch("run flags.nu -bf x", Some("x"), true)]
// A spread taken as a flag's value is the word as written, as in a call to `main`.
#[case::spread_as_a_flag_value("let l = [a b]; run flags.nu --foo ...$l", Some("...$l"), false)]
fn run_binds_main_flags(
    #[case] code: &str,
    #[case] foo: Option<&str>,
    #[case] bar: bool,
) -> Result {
    Playground::setup("run_binds_main_flags", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT),
            FileWithContentToBeTrimmed("-flags.nu", FLAGS_SCRIPT),
        ]);

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(test_record! {"foo" => foo, "bar" => bar})
    })
}

#[rstest]
#[case::unknown_flag(r#"run flags.nu --foo "23" --huh?"#, "nu::parser::unknown_flag")]
#[case::extra_positional(r#"run flags.nu --foo "23" hello"#, "nu::parser::extra_positional")]
#[case::missing_flag_value("run flags.nu --foo", "nu::parser::missing_flag_param")]
#[case::bad_switch_value("run flags.nu --bar=maybe", "nu::shell::cant_convert")]
#[case::unknown_null_flag("run flags.nu --huh=null", "nu::parser::unknown_flag")]
#[case::value_flag_inside_a_batch(
    "run flags.nu -fb x",
    "nu::parser::only_last_flag_in_batch_can_take_arg"
)]
#[case::unknown_flag_in_a_batch("run flags.nu -bx", "nu::parser::unknown_flag")]
// The parser can't tell that `--foo` takes the `--`, so it leaves `--bar=false` a word whose value
// is only text. A call to `main` binds `false`; `run` can't, but it doesn't turn `--bar` on either.
#[case::switch_value_after_a_dash_value(
    "run flags.nu --foo -- --bar=false",
    "nu::shell::cant_convert"
)]
// A record spread naming `run`'s `--full-reparse` is unknown to `main`, not `main`'s `-f`.
#[case::spread_of_a_run_flag(
    "let opts = {full-reparse: true}; run flags.nu ...$opts",
    "nu::parser::unknown_flag"
)]
#[case::full_reparse_unknown_flag("run --full-reparse flags.nu --huh?", "nu::parser::unknown_flag")]
fn run_rejects_arguments_main_does_not_accept(
    #[case] code: &str,
    #[case] error_code: &str,
) -> Result {
    Playground::setup(
        "run_rejects_arguments_main_does_not_accept",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT)]);

            test()
                .cwd(dirs.test())
                .run(code)
                .expect_error_code_eq(error_code)
        },
    )
}

#[rstest]
#[case::flag("run flags.nu --foo 23")]
#[case::positional("run word.nu 23")]
fn run_does_not_convert_script_arguments(#[case] code: &str) -> Result {
    Playground::setup("run_does_not_convert_script_arguments", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT),
            FileWithContentToBeTrimmed(
                "word.nu",
                "
                def main [word: string] { $word }
            ",
            ),
        ]);

        // The arguments are parsed without `main`'s signature, so `23` stays an `int`.
        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        assert_matches!(
            err,
            ShellError::CantConvert { to_type, help: Some(help), .. }
                if to_type == "string" && help.contains("quote the value")
        );
        Ok(())
    })
}

#[test]
fn run_script_arguments_use_the_callers_scope() -> Result {
    Playground::setup(
        "run_script_arguments_use_the_callers_scope",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "shadow.nu",
                r#"
                const name = "script"
                def helper [] { "script" }
                def main [a: string, b: string] { [$a $b] }
            "#,
            )]);

            // This parse of the script doesn't change what the arguments mean. A second `run` of
            // it in the same scope would see the definitions the first one declared.
            test()
                .cwd(dirs.test())
                .run(r#"let name = "caller"; def helper [] { "caller" }; run shadow.nu $name (helper)"#)
                .expect_value_eq(test_value!(["caller", "caller"]))
        },
    )
}

#[rstest]
// A call to `main` can't spread a list into it without a rest parameter either.
#[case::list_without_rest(
    "let args = [--bar]; run flags.nu ...$args",
    "Cannot spread a list into this command"
)]
// A known limitation, not something `main` rejects: the evaluator expands a record spread against
// `run`'s own flags before `run` sees it, so `main`'s `--foo` is unknown to it.
#[case::record_against_run("run flags.nu ...{foo: x}", "Unknown flag `foo` in spread record")]
fn run_rejects_spreads(#[case] code: &str, #[case] message: &str) -> Result {
    Playground::setup("run_rejects_spreads", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT)]);

        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        assert_contains(message, err.to_string());
        Ok(())
    })
}

#[test]
fn run_rejects_unknown_flags_before_the_script_path() -> Result {
    Playground::setup(
        "run_rejects_unknown_flags_before_the_script_path",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT)]);

            let err = test()
                .cwd(dirs.test())
                .run("run --foo x flags.nu")
                .expect_parse_error()?;
            assert_matches!(
                err,
                ParseError::UnknownFlag(command, flag, _, _) if command == "run" && flag == "foo"
            );
            Ok(())
        },
    )
}

#[rstest]
#[case::cached("run flags.nu --help")]
#[case::full_reparse("run --full-reparse flags.nu --help")]
// The help is a string even when `main` declares other input and output types.
#[case::typed_main(r#""input" | run typed.nu --help"#)]
// A record spread asks for help as `run`'s own `--help`.
#[case::record_spread("run flags.nu ...{help: true}")]
fn run_help_after_the_script_path_shows_main_help(#[case] code: &str) -> Result {
    Playground::setup(
        "run_help_after_the_script_path_shows_main_help",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContentToBeTrimmed("flags.nu", FLAGS_SCRIPT),
                FileWithContentToBeTrimmed(
                    "typed.nu",
                    "
                    def main [--foo(-f): string]: nothing -> int { 1 }
                ",
                ),
            ]);

            let help: String = test().cwd(dirs.test()).run(code)?;
            assert_contains("-f, --foo <string>", help);
            Ok(())
        },
    )
}

#[test]
fn run_binds_short_flags_like_a_call_to_main() -> Result {
    Playground::setup(
        "run_binds_short_flags_like_a_call_to_main",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "short.nu",
                "
                def main [-a, -b, --g, --other(-g), --eh(-é)] {
                    {a: $a, b: $b, g: $g, other: $other, eh: $eh}
                }
            ",
            )]);

            // `-b` is a short-only flag, `-g` is `--other`'s short, and `-é` isn't a letter.
            test()
                .cwd(dirs.test())
                .run("run short.nu -b -g -é")
                .expect_value_eq(test_record! {
                    "a" => false,
                    "b" => true,
                    "g" => false,
                    "other" => true,
                    "eh" => true,
                })
        },
    )
}

#[rstest]
#[case::words("run wrapped.nu --verbose build -- x")]
#[case::spread("let args = [--verbose build]; run wrapped.nu ...$args -- x")]
fn run_passes_unknown_flags_to_a_wrapped_main(#[case] code: &str) -> Result {
    Playground::setup(
        "run_passes_unknown_flags_to_a_wrapped_main",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "wrapped.nu",
                "
                def --wrapped main [...rest] { $rest }
            ",
            )]);

            test()
                .cwd(dirs.test())
                .run(code)
                .expect_value_eq(test_value!(["--verbose", "build", "--", "x"]))
        },
    )
}

#[test]
fn run_calls_main_with_attributes() -> Result {
    Playground::setup("run_calls_main_with_attributes", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "attr.nu",
            r#"
            @example "greet" { main --foo x }
            def main [--foo: string] { $foo }
        "#,
        )]);

        test()
            .cwd(dirs.test())
            .run("run attr.nu --foo x")
            .expect_value_eq("x")
    })
}

#[test]
fn run_main_arguments_fill_positionals_and_rest() -> Result {
    Playground::setup(
        "run_main_arguments_fill_positionals_and_rest",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "rest.nu",
                "
                def main [first: string, ...rest: int] {
                    {first: $first, rest: $rest, in: $in}
                }
            ",
            )]);

            test()
                .cwd(dirs.test())
                .run(r#"let more = [2 3]; "input" | run rest.nu a 1 ...$more"#)
                .expect_value_eq(test_value!({first: "a", rest: [1, 2, 3], in: "input"}))
        },
    )
}

#[test]
fn run_main_arguments_after_double_dash_are_positional() -> Result {
    Playground::setup(
        "run_main_arguments_after_double_dash_are_positional",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "word.nu",
                "
                def main [word: string, --flag] { $word }
            ",
            )]);

            test()
                .cwd(dirs.test())
                .run("run word.nu -- --flag")
                .expect_value_eq("--flag")
        },
    )
}

#[test]
fn run_full_reparse_flag_after_the_script_path_is_mains() -> Result {
    Playground::setup(
        "run_full_reparse_flag_after_the_script_path_is_mains",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "reparse.nu",
                "
                def main [--full-reparse: string] { $full_reparse }
            ",
            )]);

            // `run` reads its own `--full-reparse` only before the path, so this value isn't
            // evaluated as a constant for `run`.
            test()
                .cwd(dirs.test())
                .run(r#"let v = "x"; run reparse.nu --full-reparse=$v"#)
                .expect_value_eq("x")
        },
    )
}

#[rstest]
// `--foo=null` reaches `main`, so a flag that takes `null` gets it instead of its default.
#[case::null_flag("run defaults.nu --foo=null", test_value!({foo: (), bb: false}))]
// `--bb` names the flag that declares it, not an earlier one with the same short.
#[case::shared_short("run defaults.nu --bb", test_value!({foo: 5, bb: true}))]
fn run_binds_flags_by_their_long_name(#[case] code: &str, #[case] expected: Value) -> Result {
    Playground::setup("run_binds_flags_by_their_long_name", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "defaults.nu",
            "
            def main [--foo: any = 5, -a, --bb(-a)] { {foo: $foo, bb: $bb} }
        ",
        )]);

        test().cwd(dirs.test()).run(code).expect_value_eq(expected)
    })
}

#[rstest]
// Quoting a variable would pass its name, so there's no hint for it.
#[case::variable("let n = 5; run word.nu $n", None)]
#[case::number_for_a_float("run float.nu 5", Some("pass a value of type `float`"))]
fn run_type_errors_hint_only_at_literals(#[case] code: &str, #[case] hint: Option<&str>) -> Result {
    Playground::setup("run_type_errors_hint_only_at_literals", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed(
                "word.nu",
                "
                def main [word: string] { $word }
            ",
            ),
            FileWithContentToBeTrimmed(
                "float.nu",
                "
                def main [x: float] { $x }
            ",
            ),
        ]);

        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        match hint {
            None => assert_matches!(err, ShellError::CantConvert { help: None, .. }),
            Some(hint) => assert_matches!(
                err,
                ShellError::CantConvert { help: Some(help), .. } if help.contains(hint)
            ),
        }
        Ok(())
    })
}

#[test]
fn run_null_spread_into_main_without_rest_adds_nothing() -> Result {
    Playground::setup(
        "run_null_spread_into_main_without_rest_adds_nothing",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "optional.nu",
                "
                def main [word?: string] { $word }
            ",
            )]);

            test()
                .cwd(dirs.test())
                .run("let n = null; run optional.nu ...$n hello")
                .expect_value_eq("hello")
        },
    )
}

#[test]
fn run_does_not_pass_record_spread_flags_to_a_wrapped_main() -> Result {
    Playground::setup(
        "run_does_not_pass_record_spread_flags_to_a_wrapped_main",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "wrapped.nu",
                "
                def --wrapped main [...rest] { $rest }
            ",
            )]);

            // A call to `main` rejects the spread too, instead of passing the record's text on.
            test()
                .cwd(dirs.test())
                .run("let o = {help: true}; run wrapped.nu a ...$o b")
                .expect_error_code_eq("nu::parser::unknown_flag")
        },
    )
}

#[test]
fn run_counts_toward_the_recursion_limit() -> Result {
    Playground::setup("run_counts_toward_the_recursion_limit", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "rec.nu",
            r#"
            def main [n: int] {
                if $n > 0 { run --full-reparse rec.nu ($n - 1) } else { "bottom" }
            }
        "#,
        )]);

        // Each level is a call to `main`, so the limit stops the recursion before the stack
        // overflows.
        test()
            .cwd(dirs.test())
            .run("$env.config.recursion_limit = 10; run --full-reparse rec.nu 30")
            .expect_error_code_eq("nu::shell::recursion_limit_reached")
    })
}
