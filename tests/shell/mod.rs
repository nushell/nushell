use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, pipeline};
use pretty_assertions::assert_eq;

#[cfg(feature = "which-support")]
mod environment;

mod pipeline;
mod repl;

//FIXME: jt: we need to focus some fixes on wix as the plugins will differ
#[ignore]
#[test]
fn plugins_are_declared_with_wix() {
    let actual = nu!(pipeline(
        r#"
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
            "#
    ));

    assert_eq!(actual.out, "0");
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
fn nu_lib_dirs_repl() {
    Playground::setup("nu_lib_dirs_repl", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
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

        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(inp_lines));

        assert!(actual_repl.err.is_empty());
        assert_eq!(actual_repl.out, "foo");
    })
}

#[test]
fn nu_lib_dirs_script() {
    Playground::setup("nu_lib_dirs_script", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
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

        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(inp_lines));

        assert!(actual_repl.err.is_empty());
        assert_eq!(actual_repl.out, "foo");
    })
}

#[test]
fn nu_lib_dirs_relative_repl() {
    Playground::setup("nu_lib_dirs_relative_repl", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
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

        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(inp_lines));

        assert!(actual_repl.err.is_empty());
        assert_eq!(actual_repl.out, "foo");
    })
}

// TODO: add absolute path tests after we expand const capabilities (see #8310)
#[test]
fn const_nu_lib_dirs_relative() {
    Playground::setup("const_nu_lib_dirs_relative", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
                "scripts/foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                "
                    const NU_LIB_DIRS = [ 'scripts' ]
                    source-env foo.nu
                    $env.FOO
                ",
            )]);

        let outcome = nu!(cwd: dirs.test(), "source main.nu");

        assert!(outcome.err.is_empty());
        assert_eq!(outcome.out, "foo");
    })
}

#[test]
fn nu_lib_dirs_relative_script() {
    Playground::setup("nu_lib_dirs_relative_script", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
                "scripts/main.nu",
                "
                    source-env ../foo.nu
                ",
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    $env.FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            "$env.NU_LIB_DIRS = [ 'scripts' ]",
            "source-env scripts/main.nu",
            "$env.FOO",
        ];

        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(inp_lines));

        assert!(actual_repl.err.is_empty());
        assert_eq!(actual_repl.out, "foo");
    })
}

#[test]
fn run_script_that_looks_like_module() {
    Playground::setup("run_script_that_looks_like_module", |dirs, _| {
        let inp_lines = &[
            "module spam { export def eggs [] { 'eggs' } }",
            "export use spam eggs",
            "export def foo [] { eggs }",
            "export alias bar = foo",
            "export def --env baz [] { bar }",
            "baz",
        ];

        let actual = nu!(cwd: dirs.test(), inp_lines.join("; "));

        assert_eq!(actual.out, "eggs");
    })
}

#[test]
fn run_export_extern() {
    Playground::setup("run_script_that_looks_like_module", |dirs, _| {
        let inp_lines = &["export extern foo []", "help foo"];

        let actual = nu!(cwd: dirs.test(), inp_lines.join("; "));

        assert!(actual.out.contains("Usage"));
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
        .args(["-c", "echo $nu.is-login"])
        .output()
        .expect("failed to run nu");

    assert_eq!("false\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_in_interactive_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-i", "-c", "echo $nu.is-interactive"])
        .output()
        .expect("failed to run nu");

    assert_eq!("true\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_in_noninteractive_mode() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["-c", "echo $nu.is-interactive"])
        .output()
        .expect("failed to run nu");

    assert_eq!("false\n", String::from_utf8_lossy(&child_output.stdout));
    assert!(child_output.stderr.is_empty());
}

#[test]
fn run_with_no_newline() {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .args(["--no-newline", "-c", "\"hello world\""])
        .output()
        .expect("failed to run nu");

    assert_eq!("hello world", String::from_utf8_lossy(&child_output.stdout)); // with no newline
    assert!(child_output.stderr.is_empty());
}

#[test]
fn main_script_can_have_subcommands1() {
    Playground::setup("main_subcommands", |dirs, sandbox| {
        sandbox.mkdir("main_subcommands");
        sandbox.with_files(vec![FileWithContent(
            "script.nu",
            r#"def "main foo" [x: int] {
                    print ($x + 100)
                  }

                  def "main" [] {
                    print "usage: script.nu <command name>"
                  }"#,
        )]);

        let actual = nu!(cwd: dirs.test(), pipeline("nu script.nu foo 123"));

        assert_eq!(actual.out, "223");
    })
}

#[test]
fn main_script_can_have_subcommands2() {
    Playground::setup("main_subcommands", |dirs, sandbox| {
        sandbox.mkdir("main_subcommands");
        sandbox.with_files(vec![FileWithContent(
            "script.nu",
            r#"def "main foo" [x: int] {
                    print ($x + 100)
                  }

                  def "main" [] {
                    print "usage: script.nu <command name>"
                  }"#,
        )]);

        let actual = nu!(cwd: dirs.test(), pipeline("nu script.nu"));

        assert!(actual.out.contains("usage: script.nu"));
    })
}

#[test]
fn source_empty_file() {
    Playground::setup("source_empty_file", |dirs, sandbox| {
        sandbox.mkdir("source_empty_file");
        sandbox.with_files(vec![FileWithContent("empty.nu", "")]);

        let actual = nu!(cwd: dirs.test(), pipeline("nu empty.nu"));
        assert!(actual.out.is_empty());
    })
}
