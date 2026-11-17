use nu_test_support::{fs::Stub::FileWithContent, prelude::*};

/// Makes `name` in `dir` executable, for the shebang tests.
#[cfg(unix)]
fn make_executable(dir: impl AsRef<std::path::Path>, name: &str) -> Result {
    use std::os::unix::fs::PermissionsExt;

    let script = dir.as_ref().join(name);
    let mut permissions = std::fs::metadata(&script)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions)?;
    Ok(())
}

#[test]
#[deps(NU)]
fn nested_nu_piped_to_complete_keeps_raw_process() -> Result {
    let result: CompleteResult = test().run("nu -n -c '[1 2 3] | to nuon' | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_contains("[1, 2, 3]", result.stdout);
    Ok(())
}

#[test]
#[deps(NU)]
fn child_flag_false_disables_handshake() -> Result {
    test()
        .run("nu -n --structured-io=false -c '[1 2 3]' | describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn child_nu_list_stays_structured() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
fn child_nu_list_is_usable() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | math sum")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn child_nu_table_columns() -> Result {
    test()
        .run("nu -n -c '[[a b]; [1 2] [3 4]]' | columns")
        .expect_value_eq(["a", "b"])
}

#[test]
#[deps(NU)]
fn child_value_reaches_custom_command_input() -> Result {
    test()
        .run("def total [] { $in | math sum }; nu -n -c '[1 2 3]' | total")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn child_value_reaches_cell_path_on_in() -> Result {
    test().run("nu -n -c '[1 2 3]' | $in.1").expect_value_eq(2)
}

#[test]
#[deps(NU)]
fn child_value_collected_into_variable() -> Result {
    test()
        .run("let x = (nu -n -c '{a: 1kb}'); $x.a | describe")
        .expect_value_eq("filesize")
}

#[test]
#[deps(NU)]
fn parent_sends_structured_stdin() -> Result {
    test()
        .run("[1 2 3] | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn parent_sends_table_stdin() -> Result {
    test()
        .run("[[name]; [foo] [bar]] | nu -n -c '$in | get name'")
        .expect_value_eq(["foo", "bar"])
}

#[test]
#[deps(NU)]
fn nu_into_nu_passes_the_value() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn nu_chain_passes_the_value_through() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | nu -n -c '$in' | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn nu_with_fed_stdin_into_nu() -> Result {
    // The middle `nu`'s stdin is fed by a thread, so its output can't be handed on as a pipe.
    test()
        .run("[1 2 3] | nu -n -c '$in' | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[rstest::rstest]
#[case::catch("catch { 'x' }")]
#[case::finally("finally { }")]
#[case::catch_and_finally("catch { 'x' } finally { }")]
#[nu_test_support::test]
#[deps(NU)]
fn try_passes_the_value_on(#[case] handlers: &str) -> Result {
    test()
        .run(format!(
            "try {{ nu -n -c '[1 2 3]' }} {handlers} | nu -n -c '$in | math sum'"
        ))
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn custom_command_output_into_nu() -> Result {
    // The custom command's last `nu` hands its output on without waiting for it.
    test()
        .run("def three [] { nu -n -c '[1 2 3]' }; three | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn value_into_a_text_reader_is_text() -> Result {
    // The second child asks for text, so it gets `hello`, not a NUON frame of `"hello"`.
    test()
        .run("nu -n -c '\"hello\"' | nu -n --structured-io=false --stdin -c '$in'")
        .expect_value_eq("hello")
}

#[test]
#[deps(NU)]
fn print_only_child_stays_text() -> Result {
    test()
        .run("nu -n -c 'print a; print b' | lines")
        .expect_value_eq(["a", "b"])
}

#[test]
#[deps(NU)]
fn print_before_value_makes_text() -> Result {
    let out: String = test().run("nu -n -c 'print hello; [1 2 3]' | to text")?;
    assert!(out.starts_with("hello\n"), "{out:?}");
    assert!(!out.contains('\u{1e}'), "{out:?}");
    Ok(())
}

#[test]
#[deps(NU)]
fn print_to_stderr_keeps_value() -> Result {
    test()
        .run("nu -n -c 'print -e hello; [1 2 3]' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
fn external_output_before_value_makes_text() -> Result {
    let out: String = test().run("nu -n -c 'nu -n -c \"print hi\"; [1 2 3]' | to text")?;
    assert!(out.starts_with("hi\n"), "{out:?}");
    assert!(!out.contains('\u{1e}'), "{out:?}");
    Ok(())
}

#[test]
#[deps(NU)]
fn merged_stderr_stays_text() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' o+e>| describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn unserializable_value_falls_back_to_text() -> Result {
    test()
        .run("nu -n -c '{|x| $x}' | describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn unserializable_input_falls_back_to_text() -> Result {
    test()
        .run("[{|| 1}] | nu -n -c '$in | describe'")
        .expect_value_eq("string")
}

#[test]
#[deps(NU)]
fn script_output_before_main_makes_text() -> Result {
    Playground::setup("structured_io_output_before_main", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "def main [] { [1 2 3] }\n'top'\n",
        )]);

        let out: String = test().cwd(dirs.test()).run("nu -n foo.nu | to text")?;
        assert!(out.starts_with("top"), "{out:?}");
        assert!(!out.contains('\u{1e}'), "{out:?}");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn handshake_is_hidden_from_the_child() -> Result {
    test()
        .run("nu -n -c '$env.NU_STRUCTURED_IO? | describe'")
        .expect_value_eq("nothing")
}

#[test]
#[deps(NU)]
fn user_set_handshake_does_not_reach_an_uncaptured_child() -> Result {
    // `complete` reads the raw output, so the parent removes the variable and the child prints text.
    let result: CompleteResult = test()
        .run("with-env {NU_STRUCTURED_IO: '1'} { nu -n -c '[1 2 3] | to nuon' | complete }")?;
    assert_eq!(result.stdout.trim(), "[1, 2, 3]");
    Ok(())
}

#[test]
#[deps(NU)]
fn uncaptured_nested_nu_prints_text() -> Result {
    // The inner `nu` is the last command of the middle one, whose stdout `complete` reads as text.
    let result: CompleteResult =
        test().run("nu -n -c '[1 2 3] | nu -n -c \"$in | describe\"' | complete")?;
    assert_eq!(result.stdout.trim(), "list<int>");
    Ok(())
}

#[test]
#[deps(NU)]
fn child_script_file_stays_structured() -> Result {
    Playground::setup("structured_io_script_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "
                def main [] {
                    [1 2 3]
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("nu -n foo.nu | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
fn child_script_file_reads_structured_stdin() -> Result {
    Playground::setup("structured_io_script_file_stdin", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "
                def main [] {
                    $in | math sum
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("[1 2 3] | nu -n foo.nu")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
fn child_user_out_flag_does_not_take_structured_stdin() -> Result {
    test()
        .run("[1 2 3] | nu -n --structured-io=out -c '$in | describe'")
        .expect_value_eq("nothing")
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_script_stays_structured() -> Result {
    Playground::setup("structured_io_shebang", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "#!/usr/bin/env nu
def main [] {
    [1 2 3]
}
",
        )]);
        make_executable(dirs.test(), "foo.nu")?;

        test()
            .cwd(dirs.test())
            .run("./foo.nu | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_script_reads_structured_stdin() -> Result {
    Playground::setup("structured_io_shebang_stdin", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "#!/usr/bin/env nu
def main [] {
    $in | math sum
}
",
        )]);
        make_executable(dirs.test(), "foo.nu")?;

        test()
            .cwd(dirs.test())
            .run("[1 2 3] | ./foo.nu | describe")
            .expect_value_eq("int")
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_scripts_chain() -> Result {
    Playground::setup("structured_io_shebang_chain", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent(
                "list.nu",
                "#!/usr/bin/env nu
def main [] { [1 2 3] }
",
            ),
            FileWithContent(
                "sum.nu",
                "#!/usr/bin/env nu
def main [] { $in | math sum }
",
            ),
        ]);
        make_executable(dirs.test(), "list.nu")?;
        make_executable(dirs.test(), "sum.nu")?;

        test()
            .cwd(dirs.test())
            .run("./list.nu | ./sum.nu")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_script_piped_to_complete_keeps_text() -> Result {
    Playground::setup("structured_io_shebang_complete", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "#!/usr/bin/env nu
def main [] { [1 2 3] | to nuon }
",
        )]);
        make_executable(dirs.test(), "foo.nu")?;

        let result: CompleteResult = test().cwd(dirs.test()).run("./foo.nu | complete")?;
        assert_eq!(result.stdout.trim(), "[1, 2, 3]");
        Ok(())
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_script_found_on_path_stays_structured() -> Result {
    Playground::setup("structured_io_path_script", |dirs, sandbox| {
        sandbox.within("bin").with_files(&[FileWithContent(
            "sum3",
            "#!/usr/bin/env nu
def main [] { [1 2 3] }
",
        )]);
        make_executable(dirs.test().join("bin"), "sum3")?;

        test()
            .cwd(dirs.test())
            .run("$env.PATH = ($env.PATH | prepend ($env.PWD | path join bin)); sum3 | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_script_arg_false_does_not_disable_parent() -> Result {
    Playground::setup("structured_io_shebang_script_arg", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            "#!/usr/bin/env nu
def main [--structured-io: any] {
    [1 2 3]
}
",
        )]);
        make_executable(dirs.test(), "foo.nu")?;

        test()
            .cwd(dirs.test())
            .run("./foo.nu --structured-io=false | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[cfg(unix)]
fn sh_shebang_nu_extension_stays_bytes() -> Result {
    Playground::setup("structured_io_liar_nu", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "liar.nu",
            r#"#!/bin/sh
printf 'hello\n'
"#,
        )]);
        make_executable(dirs.test(), "liar.nu")?;

        test()
            .cwd(dirs.test())
            .run("./liar.nu | describe")
            .expect_value_eq("byte stream")
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn external_child_does_not_inherit_handshake() -> Result {
    // The middle `nu` is asked for a value; `env` must not see the request.
    test()
        .run("nu -n -c '^/usr/bin/env' | lines | any { str starts-with NU_STRUCTURED_IO= }")
        .expect_value_eq(false)
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn exec_into_nu_keeps_the_value() -> Result {
    test()
        .run("nu -n -c 'exec nu -n -c \"[1 2 3]\"' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn endless_stdin_is_not_read_ahead() -> Result {
    test()
        .run("^/usr/bin/yes | nu -n --stdin -c 'lines | first 3'")
        .expect_value_eq(["y", "y", "y"])
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn child_value_to_other_external_is_text() -> Result {
    test()
        .run("nu -n -c '\"hello\"' | ^/bin/cat")
        .expect_value_eq("hello")
}
