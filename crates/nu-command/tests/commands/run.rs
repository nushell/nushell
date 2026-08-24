use nu_protocol::{ParseError, ShellError, parser_path::MAX_RUN_SCRIPT_BYTES};
use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};
use std::io::Write;

#[test]
fn run_script_without_main_in_pipeline(playground: Playground) -> Result {
    playground.file("up.nu", indoc::indoc!{"
        str uppercase
    "})?;

    test()
        .cwd(playground.path())
        .run(r#""hello" | run up.nu"#)
        .expect_value_eq("HELLO")
}

#[test]
fn run_script_with_main_implicit_in(playground: Playground) -> Result {
    playground.file("main_up.nu", indoc::indoc!{"
        def main [] {
            $in | str uppercase
        }
    "})?;

    test()
        .cwd(playground.path())
        .run(r#""hello" | run main_up.nu"#)
        .expect_value_eq("HELLO")
}

#[test]
fn run_null_passes_pipeline_input_through(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run(r#""hello" | run null"#)
        .expect_value_eq("hello")
}

#[test]
fn run_script_with_main_parameters_and_flags(playground: Playground) -> Result {
    playground.file("format.nu", indoc::indoc!{"
            def main [value: string, --char: string] {
                $\"($in) ($value) ($char)\"
            }
        "})?;

        test()
            .cwd(playground.path())
            .run(r#""hello" | run format.nu "arg" --char "!" "#)
            .expect_value_eq("hello arg !")
}

#[test]
fn run_script_with_main_parameters_and_short_flags(playground: Playground) -> Result {
    playground.file("format_short.nu", indoc::indoc!{"
            def main [value: string, --char(-c): string] {
                $\"($in) ($value) ($char)\"
            }
        "})?;

        test()
            .cwd(playground.path())
            .run(r#""hello" | run format_short.nu "arg" -c "!" "#)
            .expect_value_eq("hello arg !")
}

#[test]
fn run_script_with_main_required_positional_does_not_implicitly_bind_pipeline_input(playground: Playground) -> Result {
    playground.file("needs_arg.nu", indoc::indoc!{"
            def main [value: string] {
                $value
            }
        "})?;

        let _ = test()
            .cwd(playground.path())
            .run(r#""hello" | run needs_arg.nu"#)
            .expect_error()?;
        Ok(())
}

#[test]
fn run_script_with_main_keeps_pipeline_input_in_in_when_positional_is_provided(playground: Playground) -> Result {
    playground.file("in_and_arg.nu", indoc::indoc!{"
            def main [file: path] {
                $\"($in) -> ($file)\"
            }
        "})?;

        test()
            .cwd(playground.path())
            .run(r#""stream" | run in_and_arg.nu "path.txt""#)
            .expect_value_eq("stream -> path.txt")
}

#[test]
fn run_script_with_exported_main_uses_main_entrypoint(playground: Playground) -> Result {
    playground.file("exported_main.nu", indoc::indoc!{"
            export def main [] {
                $in | str uppercase
            }
        "})?;

        test()
            .cwd(playground.path())
            .run(r#""hello" | run exported_main.nu"#)
            .expect_value_eq("HELLO")
}

#[test]
fn run_script_with_exported_env_main_uses_main_entrypoint_without_leaking_env(playground: Playground) -> Result {
    playground.file("exported_env_main.nu", indoc::indoc!{"
            export def --env main [] {
                $env.RUN_LOCAL = 'secret'
                $in | str uppercase
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester
            .run(r#""hello" | run exported_env_main.nu"#)
            .expect_value_eq("HELLO")?;
        match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
            ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
            err => Err(err.into()),
        }
}

#[test]
fn run_script_without_main_large_input_in_each(playground: Playground) -> Result {
    playground.file("double.nu", indoc::indoc!{"
            $in * 2
        "})?;

        test()
            .cwd(playground.path())
            .run("1..1000 | each { run double.nu } | math sum")
            .expect_value_eq(1001000)
}

#[test]
fn run_does_not_leak_env_from_script_without_main(playground: Playground) -> Result {
    playground.file("set_env.nu", indoc::indoc!{"
            $env.RUN_LOCAL = 'secret'
            $in
        "})?;

        let mut tester = test().cwd(playground.path());
        tester
            .run(r#""hello" | run set_env.nu"#)
            .expect_value_eq("hello")?;
        match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
            ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
            err => Err(err.into()),
        }
}

#[test]
fn run_does_not_leak_env_from_script_main(playground: Playground) -> Result {
    playground.file("set_env_main.nu", indoc::indoc!{"
        def main [] {
            $env.RUN_LOCAL = 'secret'
            $in
        }
    "})?;

    let mut tester = test().cwd(playground.path());
    tester
        .run(r#""hello" | run set_env_main.nu"#)
        .expect_value_eq("hello")?;
    match tester.run("$env.RUN_LOCAL").expect_shell_error()? {
        ShellError::CantFindColumn { col_name, .. } if col_name == "RUN_LOCAL" => Ok(()),
        err => Err(err.into()),
    }
}

#[test]
fn run_missing_script_reports_error(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run(r#""hello" | run does_not_exist.nu"#)
        .expect_parse_error()?;
    assert!(
        matches!(err, ParseError::SourcedFileNotFound(ref path, _) if path == "does_not_exist.nu"),
        "expected missing script error, got: {err:?}"
    );
    Ok(())
}

#[test]
fn run_script_parse_error_reports_error(playground: Playground) -> Result {
    playground.file("bad.nu", indoc::indoc!{"
        def main [ {
            $in
        }
    "})?;

    let _ = test()
        .cwd(playground.path())
        .run(r#""hello" | run bad.nu"#)
        .expect_parse_error()?;
    Ok(())
}

#[test]
fn run_script_runtime_error_reports_error(playground: Playground) -> Result {
    playground.file("runtime_fail.nu", indoc::indoc!{"
        def main [] {
            error make { msg: 'boom from run' }
        }
    "})?;

    let err = test()
        .cwd(playground.path())
        .run(r#""hello" | run runtime_fail.nu"#)
        .expect_error()?;
    assert_contains("boom from run", err.to_string());
    Ok(())
}

#[test]
fn run_multiple_scripts_in_pipeline(playground: Playground) -> Result {
    playground.file("up.nu", indoc::indoc!{"
        str uppercase
    "})?;
    playground.file("len.nu", indoc::indoc!{"
        def main [] {
            str length
        }
    "})?;

    test()
        .cwd(playground.path())
        .run(r#""hello" | run up.nu | run len.nu"#)
        .expect_value_eq(5)
}

#[test]
fn run_nested_pipeline_with_each(playground: Playground) -> Result {
    playground.file("up.nu", indoc::indoc!{"
        str uppercase
    "})?;
    playground.file("len.nu", indoc::indoc!{"
        def main [] {
            str length
        }
    "})?;

    test()
        .cwd(playground.path())
        .run("['a', 'bb', 'ccc'] | each { |x| $x | run up.nu | run len.nu } | math sum")
        .expect_value_eq(6)
}

#[test]
fn run_does_not_cross_script_main_between_invocations(playground: Playground) -> Result {
    playground.file("run-test1.nu", indoc::indoc!{"
            str uppercase
        "})?;
        playground.file("run-test2.nu", indoc::indoc!{"
            def main [] {
                str length
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester
            .run(r#""hello" | run run-test1.nu"#)
            .expect_value_eq("HELLO")?;
        tester
            .run(r#""hello" | run run-test2.nu"#)
            .expect_value_eq(5)?;
        tester
            .run(r#""hello" | run run-test1.nu"#)
            .expect_value_eq("HELLO")
}

#[test]
fn run_main_script_can_be_invoked_repeatedly(playground: Playground) -> Result {
    playground.file("run-test2.nu", indoc::indoc!{"
            def main [] {
                str length
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester
            .run(r#""hello" | run run-test2.nu"#)
            .expect_value_eq(5)?;
        tester
            .run(r#""hello" | run run-test2.nu"#)
            .expect_value_eq(5)?;
        tester
            .run(r#""hello" | run run-test2.nu"#)
            .expect_value_eq(5)
}

#[test]
fn run_main_script_tracks_file_edits_in_repl_session(playground: Playground) -> Result {
    playground.file("run-edit.nu", indoc::indoc!{"
            def main [] {
                'hello'
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester.run("run run-edit.nu").expect_value_eq("hello")?;
        tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
        tester
            .run("run run-edit.nu")
            .expect_value_eq("hello world")?;
        tester.run::<()>(r#"'def main [] { "hello" }' | save --force run-edit.nu"#)?;
        tester.run("run run-edit.nu").expect_value_eq("hello")
}

#[test]
fn run_main_script_in_reused_closure_keeps_cached_parse_by_default(playground: Playground) -> Result {
    playground.file("run-edit.nu", indoc::indoc!{"
            def main [] {
                'hello'
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester.run::<()>("let runner = { run run-edit.nu }")?;
        tester.run("do $runner").expect_value_eq("hello")?;
        tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
        tester.run("do $runner").expect_value_eq("hello")
}

#[test]
fn run_main_script_in_reused_closure_reloads_with_full_reparse(playground: Playground) -> Result {
    playground.file("run-edit.nu", indoc::indoc!{"
            def main [] {
                'hello'
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester.run::<()>("let runner = { run --full-reparse run-edit.nu }")?;
        tester.run("do $runner").expect_value_eq("hello")?;
        tester.run::<()>(r#"'def main [] { "hello world" }' | save --force run-edit.nu"#)?;
        tester.run("do $runner").expect_value_eq("hello world")?;
        tester.run::<()>(r#"'def main [] { "hello again" }' | save --force run-edit.nu"#)?;
        tester.run("do $runner").expect_value_eq("hello again")
}

#[test]
fn run_script_without_main_tracks_file_edits_with_full_reparse(playground: Playground) -> Result {
    playground.file("run-no-main.nu", indoc::indoc!{"
            str uppercase
        "})?;

        let mut tester = test().cwd(playground.path());
        tester
            .run(r#""hello" | run --full-reparse run-no-main.nu"#)
            .expect_value_eq("HELLO")?;
        tester.run::<()>("'str downcase' | save --force run-no-main.nu")?;
        tester
            .run(r#""HELLO" | run --full-reparse run-no-main.nu"#)
            .expect_value_eq("hello")
}

#[test]
fn run_full_reparse_recovers_after_script_parse_error(playground: Playground) -> Result {
    playground.file("run-edit.nu", indoc::indoc!{"
            def main [] {
                'ok'
            }
        "})?;

        let mut tester = test().cwd(playground.path());
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
}

#[test]
fn run_full_reparse_forwards_main_arguments_and_flags(playground: Playground) -> Result {
    playground.file("format.nu", indoc::indoc!{"
            def main [value: string, --char(-c): string] {
                $\"($in) ($value) ($char)\"
            }
        "})?;

        test()
            .cwd(playground.path())
            .run(r#""hello" | run --full-reparse format.nu "arg" -c "!" "#)
            .expect_value_eq("hello arg !")
}

#[test]
fn run_script_with_toolkit_like_exports_can_be_run_twice_in_repl_session(playground: Playground) -> Result {
    // Regression: running a toolkit-style script (with exports) must not break
    // subsequent `run` invocations in the same REPL session. The export must not
    // be named `run` — that is a parser keyword and is rejected.
    playground.dir("toolkit")?;
        playground.file("toolkit/wrappers.nu", indoc::indoc!{"
            export def dev [--experimental-options: string] {
                'toolkit dev'
            }
        "})?;
        playground.file("toolkit/mod.nu", indoc::indoc!{"
            export use wrappers.nu *
            
            export def main [] {
                'toolkit main'
            }
        "})?;
        playground.file("toolkit.nu", indoc::indoc!{"
            export use toolkit *
            
            export def main [] {
                help toolkit
                'ok'
            }
        "})?;

        let mut tester = test().cwd(playground.path());
        tester.run("run toolkit.nu").expect_value_eq("ok")?;
        tester.run("run toolkit.nu").expect_value_eq("ok")
}

#[test]
fn run_script_binds_long_flag_by_name_not_declaration_order(playground: Playground) -> Result {
    playground.file("flags.nu", indoc::indoc!{"
            def main [--alpha: int, --beta: int, --gamma: int] {
                $\"($alpha | default 0)/($beta | default 0)/($gamma | default 0)\"
            }
        "})?;

        // `--gamma` must bind to `--gamma` by name. Previously a long flag
        // matched the first declared flag that had no short character
        // (`--alpha`), so the value silently landed in the wrong slot.
        let mut tester = test().cwd(playground.path());
        tester
            .run("run flags.nu --gamma 3")
            .expect_value_eq("0/0/3")
}

#[test]
fn run_script_binds_switch_by_name_without_shifting_positional(playground: Playground) -> Result {
    playground.file("switch.nu", indoc::indoc!{"
            def main [word: string, --num: int, --verbose] {
                $\"word=($word) num=($num | default 0) verbose=($verbose)\"
            }
        "})?;

        // `--verbose` is a switch declared after the value-taking `--num`.
        // It must bind by name; otherwise it matched `--num`, which then
        // swallowed `hello` as its (int) value and left `word` unbound.
        let mut tester = test().cwd(playground.path());
        tester
            .run("run switch.nu hello --verbose")
            .expect_value_eq("word=hello num=0 verbose=true")
}

/// Oversized paths must not be loaded by `run` (REPL hang / multi-GiB RAM; #18597).
#[test]
fn run_oversized_file_errors_without_loading(playground: Playground) -> Result {
    let path = playground.path().join("huge.bin");
    let file = std::fs::File::create(&path).expect("create huge.bin");
    // Sparse size only — do not write MAX_RUN_SCRIPT_BYTES of data.
    file.set_len(MAX_RUN_SCRIPT_BYTES + 1)
        .expect("set oversized length");

    let err = test()
        .cwd(playground.path())
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
}

/// Binary files must be rejected before the Nu parser runs (#18597).
#[test]
fn run_binary_file_with_nul_errors_without_parsing(playground: Playground) -> Result {
    let path = playground.path().join("binary.bin");
        let mut file = std::fs::File::create(&path).expect("create binary.bin");
        file.write_all(b"not\0a\0script")
            .expect("write binary content");

        let err = test()
            .cwd(playground.path())
            .run("run binary.bin")
            .expect_parse_error()?;
        assert!(
            matches!(err, ParseError::ScriptFileNotText { .. }),
            "expected ScriptFileNotText, got: {err:?}"
        );
        Ok(())
}

/// Invalid UTF-8 (no NULs) must also be rejected as non-text for `run`.
#[test]
fn run_invalid_utf8_file_errors_without_parsing(playground: Playground) -> Result {
    let path = playground.path().join("bad_utf8.bin");
    // Lone continuation bytes: invalid UTF-8, no NULs.
    std::fs::write(&path, [0x80, 0x81, 0x82, 0x83, 0xFF]).expect("write invalid utf-8");

    let err = test()
        .cwd(playground.path())
        .run("run bad_utf8.bin")
        .expect_parse_error()?;
    assert!(
        matches!(err, ParseError::ScriptFileNotText { .. }),
        "expected ScriptFileNotText, got: {err:?}"
    );
    Ok(())
}

/// Dense C0 control characters (no NULs, valid UTF-8 bytes) look like binary to `run`.
#[test]
fn run_control_heavy_file_errors_without_parsing(playground: Playground) -> Result {
    let path = playground.path().join("controls.bin");
        // Mostly BEL/SOH-style controls; still valid UTF-8 single bytes, no NULs.
        let mut bytes = vec![0x01u8; 100];
        bytes.extend_from_slice(b"\n");
        std::fs::write(&path, bytes).expect("write control-heavy file");

        let err = test()
            .cwd(playground.path())
            .run("run controls.bin")
            .expect_parse_error()?;
        assert!(
            matches!(err, ParseError::ScriptFileNotText { .. }),
            "expected ScriptFileNotText, got: {err:?}"
        );
        Ok(())
}

/// `--full-reparse` skips parse-time load, so oversized files must still be rejected at runtime.
#[test]
fn run_full_reparse_oversized_file_errors(playground: Playground) -> Result {
    let path = playground.path().join("huge.bin");
    let file = std::fs::File::create(&path).expect("create huge.bin");
    file.set_len(MAX_RUN_SCRIPT_BYTES + 1)
        .expect("set oversized length");

    let err = test()
        .cwd(playground.path())
        .run("run --full-reparse huge.bin")
        .expect_shell_error()?;
    assert_contains("too large", err.to_string());
    Ok(())
}

