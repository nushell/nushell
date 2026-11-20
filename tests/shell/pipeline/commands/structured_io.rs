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
fn child_without_the_flag_gives_text() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn child_list_is_a_value() -> Result {
    test()
        .run("nu -n --structured-io -c '[1 2 3]' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
fn child_table_keeps_its_columns() -> Result {
    test()
        .run("nu -n --structured-io -c '[[a b]; [1 2] [3 4]]' | columns")
        .expect_value_eq(["a", "b"])
}

#[test]
#[deps(NU)]
fn child_value_keeps_its_type() -> Result {
    test()
        .run("let x = (nu -n --structured-io -c '{a: 1kb}'); $x.a | describe")
        .expect_value_eq("filesize")
}

#[test]
#[deps(NU)]
fn child_value_reaches_custom_command_input() -> Result {
    test()
        .run("def total [] { $in | math sum }; nu -n --structured-io -c '[1 2 3]' | total")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn parent_sends_its_input() -> Result {
    test()
        .run("[[name]; [foo] [bar]] | nu -n --structured-io -c '$in | get name'")
        .expect_value_eq(["foo", "bar"])
}

#[test]
#[deps(NU)]
fn byte_stream_input_stays_bytes() -> Result {
    test()
        .run("nu -n -c 'print -n \"x\ny\"' | nu -n --structured-io -c '$in | lines'")
        .expect_value_eq(["x", "y"])
}

#[test]
#[deps(NU)]
fn nu_chain_passes_values() -> Result {
    test()
        .run(
            "nu -n --structured-io -c '[1 2 3]' \
            | nu -n --structured-io -c '$in | each { $in * 2 }' \
            | nu -n --structured-io -c '$in | math sum'",
        )
        .expect_value_eq(12)
}

#[test]
#[deps(NU)]
fn endless_output_stops_when_the_parent_does() -> Result {
    test()
        .run("nu -n --structured-io -c '1.. | each {|x| $x }' | first 3")
        .expect_value_eq([1, 2, 3])
}

#[test]
#[deps(NU)]
fn endless_input_stops_when_the_child_does() -> Result {
    test()
        .run("1.. | nu -n --structured-io -c '$in | first 3'")
        .expect_value_eq([1, 2, 3])
}

#[test]
#[deps(NU)]
fn children_run_side_by_side() -> Result {
    test()
        .run("1..4 | par-each {|i| nu -n --structured-io -c $'($i * 10)' } | sort")
        .expect_value_eq([10, 20, 30, 40])
}

#[test]
#[deps(NU)]
fn print_goes_to_the_terminal() -> Result {
    // The outer `nu` stands in for the terminal.
    let result: CompleteResult = test().run(
        r#"nu -n -c "nu -n --structured-io -c 'print hello; [1 2 3]' | describe" | complete"#,
    )?;
    assert_eq!(result.exit_code, 0, "{}", result.stderr);
    assert_eq!(
        result.stdout.lines().collect::<Vec<_>>(),
        ["hello", "list<int>"]
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn trailing_external_is_a_byte_stream() -> Result {
    test()
        .run("nu -n --structured-io -c 'nu -n -c \"print a; print b\"' | lines")
        .expect_value_eq(["a", "b"])
}

#[test]
#[deps(NU)]
fn failing_trailing_external_fails() -> Result {
    test()
        .run("try { nu -n --structured-io -c 'nu -n -c \"exit 3\"' | lines } catch { 'failed' }")
        .expect_value_eq("failed")
}

#[test]
#[deps(NU)]
fn exit_without_a_result_gives_nothing() -> Result {
    test()
        .run("nu -n --structured-io -c 'exit 0' | describe")
        .expect_value_eq("nothing")
}

#[test]
#[deps(NU)]
fn failing_child_fails() -> Result {
    test()
        .run("try { nu -n --structured-io -c 'exit 3' } catch { 'failed' }")
        .expect_value_eq("failed")
}

#[test]
#[deps(NU)]
fn value_without_nuon_is_an_error() -> Result {
    test()
        .run("try { nu -n --structured-io -c '{|| 1}' } catch {|e| $e.msg }")
        .expect_value_eq("Error from another nu")
}

#[test]
#[deps(NU)]
fn error_item_ends_up_in_the_list() -> Result {
    let code = "nu -n --structured-io -c '[1 2] | each {|x| if $x == 2 { error make {msg: two} } else { $x } }'";
    test().run(format!("{code} | first")).expect_value_eq(1)?;
    test()
        .run(format!(
            "try {{ {code} | to nuon }} catch {{|e| $e.debug | str contains two }}"
        ))
        .expect_value_eq(true)
}

#[test]
#[deps(NU)]
fn try_passes_the_value_on() -> Result {
    test()
        .run("try { nu -n --structured-io -c '[1 2 3]' } catch { 'x' } | math sum")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
fn complete_gets_text() -> Result {
    let result: CompleteResult =
        test().run("nu -n --structured-io -c '[1 2 3] | to nuon' | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "[1, 2, 3]");
    Ok(())
}

#[test]
#[deps(NU)]
fn merged_stderr_gets_text() -> Result {
    test()
        .run("nu -n --structured-io -c '[1 2 3]' o+e>| describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn value_for_another_program_is_text() -> Result {
    test()
        .run(
            "nu -n --structured-io -c '\"hello\"' | nu -n --stdin -c '$in | str length' | into int",
        )
        .expect_value_eq(5)
}

#[test]
#[deps(NU)]
fn handshake_is_hidden_from_the_child() -> Result {
    test()
        .run("nu -n --structured-io -c '$env | columns | any { $in == NU_STRUCTURED_IO }'")
        .expect_value_eq(false)
}

#[test]
#[deps(NU)]
fn user_set_handshake_never_reaches_a_child() -> Result {
    test()
        .run("with-env {NU_STRUCTURED_IO: '1;x;o;nowhere'} { nu -n -c '[1 2 3]' | describe }")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
fn interactive_child_passes_values() -> Result {
    test()
        .run("[1 2 3] | nu -n -i --structured-io -c '$in | math sum' | describe")
        .expect_value_eq("int")
}

#[test]
#[deps(NU)]
fn script_file_passes_values() -> Result {
    Playground::setup("structured_io_script_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "double.nu",
            "def main [] { $in | each { $in * 2 } }",
        )]);

        test()
            .cwd(dirs.test())
            .run("[1 2 3] | nu -n --structured-io double.nu | math sum")
            .expect_value_eq(12)
    })
}

#[test]
#[deps(NU)]
fn startup_files_print_as_usual() -> Result {
    Playground::setup("structured_io_config", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "config.nu",
            "print 'from config'; 'config result'",
        )]);
        let result: CompleteResult = test().cwd(dirs.test()).run(
            r#"nu -n -c "nu --no-std-lib --config config.nu --structured-io -c '[1 2]' | describe" | complete"#,
        )?;
        assert_eq!(result.exit_code, 0, "{}", result.stderr);
        assert_eq!(result.stdout.lines().last(), Some("list<int>"));
        assert_contains("from config", result.stdout);
        Ok(())
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn exec_into_nu_keeps_the_value() -> Result {
    test()
        .run("nu -n --structured-io -c 'exec nu -n -c \"[1 2 3]\"' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn shebang_scripts_with_the_flag_chain() -> Result {
    Playground::setup("structured_io_shebang_chain", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent(
                "list.nu",
                "#!/usr/bin/env -S nu -n --structured-io
def main [] { [1 2 3] }
",
            ),
            FileWithContent(
                "sum.nu",
                "#!/usr/bin/env -S nu -n --structured-io
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
fn shebang_without_the_flag_gives_text() -> Result {
    Playground::setup("structured_io_plain_shebang", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "list.nu",
            "#!/usr/bin/env nu
def main [] { [1 2 3] }
",
        )]);
        make_executable(dirs.test(), "list.nu")?;

        test()
            .cwd(dirs.test())
            .run("./list.nu | describe")
            .expect_value_eq("byte stream")
    })
}

#[test]
#[cfg(unix)]
fn flag_for_another_program_changes_nothing() -> Result {
    // Like `rg --structured-io src`: only `nu` asks for structured IO.
    Playground::setup("structured_io_other_program", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "hello",
            "#!/bin/sh
printf 'hello\\n'
",
        )]);
        make_executable(dirs.test(), "hello")?;

        test()
            .cwd(dirs.test())
            .run("./hello --structured-io | str trim")
            .expect_value_eq("hello")
    })
}

#[test]
#[deps(NU)]
#[cfg(unix)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "only Linux finds the opener on PATH; elsewhere this would open a real application"
)]
fn start_does_not_pass_the_link_on() -> Result {
    // `start` runs the opener with this process's environment. On Linux the opener is found on
    // `PATH` (on macOS it's `/usr/bin/open`), so a fake one can record what it got. It only uses
    // shell builtins, since the test's `PATH` holds just the `nu` under test.
    Playground::setup("structured_io_start", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent(
                "xdg-open",
                "#!/bin/sh\nprintf '%s' \"${NU_STRUCTURED_IO-unset}\" > \"$PWD/opener-env\"\n",
            ),
            FileWithContent("doc.txt", "x"),
        ]);
        make_executable(dirs.test(), "xdg-open")?;

        // `describe` reads the result, so the child gets the link.
        test()
            .cwd(dirs.test())
            .run(
                "$env.PATH = ($env.PATH | prepend $env.PWD)
                nu -n --structured-io -c 'start doc.txt; 1' | describe | ignore
                open opener-env",
            )
            .expect_value_eq("unset")
    })
}
