use nu_test_support::{
    fs::Stub::{FileWithContent, FileWithContentToBeTrimmed},
    nu_repl_code,
    prelude::*,
};
use nu_utils::time::Instant;
use pretty_assertions::assert_eq;
use rstest::rstest;

mod environment;
mod pipeline;
mod repl;

//FIXME: jt: we need to focus some fixes on wix as the plugins will differ
#[ignore]
#[test]
fn plugins_are_declared_with_wix() -> Result {
    let code = r#"
        open Cargo.toml
        | get bin.name
        | str replace "nu_plugin_(extra|core)_(.*)" "nu_plugin_$2"
        | drop
        | sort-by
        | wrap cargo | merge {
            open wix/main.wxs --raw | from xml
            | get Wix.children.Product.children.0.Directory.children.0
            | where Directory.attributes.Id == "$(var.PlatformProgramFilesFolder)"
            | get Directory.children.Directory.children.0 | last
            | get Directory.children.Component.children
            | each { |it| echo $it | first }
            | skip
            | where File.attributes.Name =~ "nu_plugin"
            | str substring [_, -4] File.attributes.Name
            | get File.attributes.Name
            | sort-by
            | wrap wix
        }
        | default wix _
        | each { |it| if $it.wix != $it.cargo { 1 } { 0 } }
        | math sum
    "#;

    test().run(code).expect_value_eq(0)
}

#[test]
#[cfg(not(windows))]
fn do_not_panic_if_broken_pipe() {
    // `nu -h | false`
    // used to panic with a BrokenPipe error
    let child_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{:?} -h | false",
            nu_test_support::fs::executable_path()
        ))
        .output()
        .expect("failed to execute process");

    assert!(child_output.stderr.is_empty());
}

#[test]
#[cfg(unix)]
fn exit_failure_if_stdout_full() {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{:?} > /dev/full",
            nu_test_support::fs::executable_path()
        ))
        .spawn()
        .expect("failed to spawn process");

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("failed to query child status") {
            break status;
        }

        if start.elapsed() > std::time::Duration::from_secs(5) {
            let _ = child.kill();
            panic!("child did not exit within 5 seconds");
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    assert!(!status.success(), "expected failure status");
    assert!(
        status.code().is_some(),
        "expected process to exit normally rather than by signal"
    );
}

#[test]
#[cfg(unix)]
fn exit_failure_if_stderr_full() {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{:?} 2>/dev/full",
            nu_test_support::fs::executable_path()
        ))
        .spawn()
        .expect("failed to spawn process");

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("failed to query child status") {
            break status;
        }

        if start.elapsed() > std::time::Duration::from_secs(5) {
            let _ = child.kill();
            panic!("child did not exit within 5 seconds");
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    assert!(!status.success(), "expected failure status");
    assert!(
        status.code().is_some(),
        "expected process to exit normally rather than by signal"
    );
}

#[test]
fn nu_lib_dirs_repl() -> Result {
    Playground::setup("nu_lib_dirs_repl", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("scripts")
            .with_files(&[FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            "$env.NU_LIB_DIRS = [ ('scripts' | path expand) ]",
            "source-env foo.nu",
            "$env.FOO",
        ];

        let command = format!("{} | to text | str trim", nu_repl_code(inp_lines));
        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run(command)
            .expect_value_eq("foo")
    })
}

#[test]
fn nu_lib_dirs_script() -> Result {
    Playground::setup("nu_lib_dirs_script", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("scripts")
            .with_files(&[FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    source-env foo.nu
                ",
            )]);

        let inp_lines = &[
            "$env.NU_LIB_DIRS = [ ('scripts' | path expand) ]",
            "source-env main.nu",
            "$env.FOO",
        ];

        let command = format!("{} | to text | str trim", nu_repl_code(inp_lines));
        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run(command)
            .expect_value_eq("foo")
    })
}

#[test]
fn nu_lib_dirs_relative_repl() -> Result {
    Playground::setup("nu_lib_dirs_relative_repl", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("scripts")
            .with_files(&[FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            "$env.NU_LIB_DIRS = [ 'scripts' ]",
            "source-env foo.nu",
            "$env.FOO",
        ];

        let command = format!("{} | to text | str trim", nu_repl_code(inp_lines));
        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run(command)
            .expect_value_eq("foo")
    })
}

// TODO: add absolute path tests after we expand const capabilities (see #8310)
#[test]
fn const_nu_lib_dirs_relative() -> Result {
    Playground::setup("const_nu_lib_dirs_relative", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("scripts")
            .with_files(&[FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    const NU_LIB_DIRS = [ 'scripts' ]
                    source-env foo.nu
                    $env.FOO
                ",
            )]);

        test()
            .cwd(dirs.test())
            .run("source main.nu")
            .expect_value_eq("foo")
    })
}

#[test]
fn nu_lib_dirs_relative_script() -> Result {
    Playground::setup("nu_lib_dirs_relative_script", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("scripts")
            .with_files(&[FileWithContentToBeTrimmed(
                "scripts/main.nu",
                "
                    source-env ../foo.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )]);

        test()
            .cwd(dirs.test())
            .run("$env.NU_LIB_DIRS = [ 'scripts' ]; source-env scripts/main.nu; $env.FOO")
            .expect_value_eq("foo")
    })
}

#[test]
fn run_script_that_looks_like_module() -> Result {
    Playground::setup("run_script_that_looks_like_module", |dirs, _| {
        let mut tester = test().cwd(dirs.test());
        let () = tester.run("module spam { export def eggs [] { 'eggs' } }")?;
        let () = tester.run("export use spam eggs")?;
        let () = tester.run("export def foo [] { eggs }")?;
        let () = tester.run("export alias bar = foo")?;
        let () = tester.run("export def --env baz [] { bar }")?;
        tester.run("baz").expect_value_eq("eggs")
    })
}

#[test]
fn run_export_extern() -> Result {
    Playground::setup("run_script_that_looks_like_module", |dirs, _| -> Result {
        let code = "export extern foo []; help foo | to text";
        let help_text: String = test().cwd(dirs.test()).run(code)?;
        assert_contains("Usage", help_text);
        Ok(())
    })
}

#[test]
fn run_in_login_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-n", "-l", "-c", "echo $nu.is-login"])
        .output()
        .expect("failed to run nu");

    assert_eq!("true\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_in_not_login_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-n", "-c", "echo $nu.is-login"])
        .output()
        .expect("failed to run nu");

    assert_eq!("false\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_in_interactive_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-n", "-i", "-c", "echo $nu.is-interactive"])
        .output()
        .expect("failed to run nu");

    assert_eq!("true\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_in_noninteractive_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-n", "-c", "echo $nu.is-interactive"])
        .output()
        .expect("failed to run nu");

    assert_eq!("false\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_with_no_newline() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-n", "--no-newline", "-c", "\"hello world\""])
        .output()
        .expect("failed to run nu");

    assert_eq!("hello world", String::from_utf8_lossy(&child_output.stdout)); // with no newline
    assert!(child_output.stderr.is_empty());
}

#[test]
fn main_script_can_have_subcommands1() -> Result {
    Playground::setup("main_subcommands", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_subcommands");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            r#"def "main foo" [x: int] {
                    print ($x + 100)
                  }

                  def "main" [] {
                    print "usage: script.nu <command name>"
                  }"#,
        )]);

        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu foo 123 | to text | str trim")
            .expect_value_eq("223")
    })
}

#[test]
fn main_script_can_have_subcommands2() -> Result {
    Playground::setup("main_subcommands", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_subcommands");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            r#"def "main foo" [x: int] {
                    print ($x + 100)
                  }

                  def "main" [] {
                    print "usage: script.nu <command name>"
                  }"#,
        )]);

        let out: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu | to text")?;

        assert_contains("usage: script.nu", out);
        Ok(())
    })
}

#[test]
fn script_with_newline_arg_does_not_split_commands() -> Result {
    Playground::setup("script_newline_arg", |dirs, sandbox| -> Result {
        sandbox.mkdir("script_newline_arg");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "def main [...args: string] { print ...($args) }",
        )]);

        // If newline escaping regresses, parsing fails before returning "ok".
        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu a b \"c\\nd\"; 'ok'")
            .expect_value_eq("ok")
    })
}

// regression test for https://github.com/nushell/nushell/issues/17719
#[test]
fn script_help_shows_single_subcommand() -> Result {
    Playground::setup("main_subcommands_help", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_subcommands_help");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            r#"def "main bar" [] {}
               def "main" [] { help main }"#,
        )]);

        let out: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu --help | to text")?;
        let count_script = out.matches("script.nu bar").count();
        let count_main = out.matches("main bar").count();
        assert_eq!(
            count_script + count_main,
            1,
            "help output should list exactly one of 'script.nu bar' or 'main bar', got:\n{}",
            out
        );

        Ok(())
    })
}

#[test]
fn source_empty_file() -> Result {
    Playground::setup("source_empty_file", |dirs, sandbox| -> Result {
        sandbox.mkdir("source_empty_file");
        sandbox.with_files(&[FileWithContent("empty.nu", "")]);

        let out: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu empty.nu | to text")?;
        assert!(out.is_empty());
        Ok(())
    })
}

#[rstest]
#[case("source null; null | describe")]
#[case("source-env null; null | describe")]
#[case("use null; null | describe")]
#[case("overlay use null; null | describe")]
fn source_use_null(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("nothing")
}

#[test]
fn source_use_file_named_null() -> Result {
    Playground::setup("source_file_named_null", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "null",
            r#"export-env { $env.NULL_TEST_GREETING = "hello world" }"#,
        )]);

        test()
            .cwd(dirs.test())
            .run(r#"source "null"; $env.NULL_TEST_GREETING"#)
            .expect_value_eq("hello world")?;
        test()
            .cwd(dirs.test())
            .run(r#"source-env "null"; $env.NULL_TEST_GREETING"#)
            .expect_value_eq("hello world")?;
        test()
            .cwd(dirs.test())
            .run(r#"use "null"; $env.NULL_TEST_GREETING"#)
            .expect_value_eq("hello world")?;
        test()
            .cwd(dirs.test())
            .run(r#"overlay use "null"; $env.NULL_TEST_GREETING"#)
            .expect_value_eq("hello world")?;

        Ok(())
    })
}

#[test]
fn main_script_help_uses_script_name1() -> Result {
    // Note: this test is somewhat fragile and might need to be adapted if the usage help message changes
    Playground::setup("main_filename1", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_filename1");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "def main [] {}
            ",
        )]);
        let out: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu --help | to text")?;
        assert!(out.contains("> script.nu"));
        assert!(!out.contains("> main"));
        Ok(())
    })
}

#[test]
fn main_script_help_uses_script_name2() -> Result {
    // Note: this test is somewhat fragile and might need to be adapted if the usage help message changes
    Playground::setup("main_filename2", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_filename2");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "def main [foo: string] {}
            ",
        )]);
        let stderr: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu | complete | get stderr")?;
        assert!(stderr.contains("Usage: script.nu"));
        assert!(!stderr.contains("Usage: main"));
        Ok(())
    })
}

#[test]
fn main_script_subcommand_help_uses_script_name1() -> Result {
    // Note: this test is somewhat fragile and might need to be adapted if the usage help message changes
    Playground::setup("main_filename3", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_filename3");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "def main [] {}
            def 'main foo' [] {}
            ",
        )]);
        let out: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu foo --help | to text")?;
        assert!(out.contains("> script.nu foo"));
        assert!(!out.contains("> main foo"));
        Ok(())
    })
}

#[test]
fn main_script_subcommand_help_uses_script_name2() -> Result {
    // Note: this test is somewhat fragile and might need to be adapted if the usage help message changes
    Playground::setup("main_filename4", |dirs, sandbox| -> Result {
        sandbox.mkdir("main_filename4");
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "def main [] {}
            def 'main foo' [bar: string] {}
            ",
        )]);
        let stderr: String = test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu foo | complete | get stderr")?;
        assert!(stderr.contains("Usage: script.nu foo"));
        assert!(!stderr.contains("Usage: main foo"));
        Ok(())
    })
}

#[test]
fn script_file_not_found() -> Result {
    let stderr: String = test()
        .add_nu_to_path()
        .run("nu non-existent-script.nu foo bar | complete | get stderr")?;
    assert!(
        !stderr.contains(".rs"),
        "internal rust source was mentioned"
    );
    assert!(
        stderr.contains("non-existent-script.nu"),
        "error did not include script name"
    );
    assert!(
        stderr.contains("commandline"),
        "source file for the error was not commandline"
    );
    Ok(())
}

#[test]
fn main_script_alias_persists() -> Result {
    // Verify that renaming 'main' to the script filename doesn't prevent the
    // script from running correctly via its filename as the command name.
    Playground::setup("alias_main", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("script.nu", "def main [] { 'ok' }")]);

        test()
            .cwd(dirs.test())
            .add_nu_to_path()
            .run("nu script.nu | to text | str trim")
            .expect_value_eq("ok")
    })
}

// This test will have to change once clip copy is removed after deprecation time.
#[test]
#[exp(nu_experimental::NATIVE_CLIP)]
fn builtin_commands_can_be_shadowed_and_extended() -> Result {
    // Demonstrate that importing a module can shadow built-in commands and
    // add new subcommands, which is the motivating use case for this PR.
    let outcome: String = test().run("use std/clip; clip")?;
    assert_contains("clip copy52", &outcome);
    assert_contains("clip prefix", &outcome);
    assert_contains("clip copy ", &outcome);
    assert_eq!(outcome.matches("clip copy ").count(), 1);

    let outcome: String = test().run("use std/clip; clip copy --help")?;
    assert_contains("deprecated", outcome);

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn nu_env_pwd_symlink() {
    Playground::setup("nu_env_pwd_symlink", |_, sandbox| {
        // Test that the value of PWD in the environment takes precedence
        // over the current working directory when they point to the same directory.
        let pwd = "linked_current_dir";
        sandbox.symlink("./", pwd);

        let pwd = sandbox.cwd().join(pwd);
        let current_dir = std::fs::canonicalize(&pwd).unwrap();
        let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
            .args(["-c", "echo $env.PWD"])
            .current_dir(current_dir)
            .env("PWD", &pwd)
            .output()
            .expect("failed to run nu");
        let output = String::from_utf8(child_output.stdout).unwrap();
        assert_eq!(output.trim_end(), pwd.to_str().unwrap());

        // Make sure that the current_dir still takes precedence
        // if PWD and current_dir point to different directories.
        let pwd = "linked_current_dir2";
        sandbox.mkdir("new_current_dir");
        sandbox.symlink("new_current_dir", pwd);

        let pwd = sandbox.cwd().join(pwd);
        let current_dir = sandbox.cwd().canonicalize().unwrap();
        let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
            .args(["-c", "echo $env.PWD"])
            .current_dir(&current_dir)
            .env("PWD", &pwd)
            .output()
            .expect("failed to run nu");
        let output = String::from_utf8(child_output.stdout).unwrap();
        assert_eq!(output.trim_end(), current_dir.to_str().unwrap());
    })
}
