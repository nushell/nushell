use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, pipeline};

#[cfg(feature = "which-support")]
mod environment;

mod pipeline;

//FIXME: jt: we need to focus some fixes on wix as the plugins will differ
#[ignore]
#[test]
fn plugins_are_declared_with_wix() {
    let actual = nu!(
        cwd: ".", pipeline(
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
                    let-env FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            r#"let-env NU_LIB_DIRS = [ ('scripts' | path expand) ]"#,
            r#"source-env foo.nu"#,
            r#"$env.FOO"#,
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
                    let-env FOO = "foo"
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    source-env foo.nu
                "#,
            )]);

        let inp_lines = &[
            r#"let-env NU_LIB_DIRS = [ ('scripts' | path expand) ]"#,
            r#"source-env main.nu"#,
            r#"$env.FOO"#,
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
                    let-env FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            r#"let-env NU_LIB_DIRS = [ 'scripts' ]"#,
            r#"source-env foo.nu"#,
            r#"$env.FOO"#,
        ];

        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(inp_lines));

        assert!(actual_repl.err.is_empty());
        assert_eq!(actual_repl.out, "foo");
    })
}

#[test]
fn nu_lib_dirs_relative_script() {
    Playground::setup("nu_lib_dirs_relative_script", |dirs, sandbox| {
        sandbox
            .mkdir("scripts")
            .with_files(vec![FileWithContentToBeTrimmed(
                "scripts/main.nu",
                r#"
                    source-env ../foo.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "foo.nu",
                r#"
                    let-env FOO = "foo"
                "#,
            )]);

        let inp_lines = &[
            r#"let-env NU_LIB_DIRS = [ 'scripts' ]"#,
            r#"source-env scripts/main.nu"#,
            r#"$env.FOO"#,
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
            r#"module spam { export def eggs [] { 'eggs' } }"#,
            r#"export use spam eggs"#,
            r#"export def foo [] { eggs }"#,
            r#"export alias bar = foo"#,
            r#"export def-env baz [] { bar }"#,
            r#"baz"#,
        ];

        let actual = nu!(cwd: dirs.test(), inp_lines.join("; "));

        assert_eq!(actual.out, "eggs");
    })
}

#[test]
fn run_export_extern() {
    Playground::setup("run_script_that_looks_like_module", |dirs, _| {
        let inp_lines = &[r#"export extern foo []"#, r#"help foo"#];

        let actual = nu!(cwd: dirs.test(), inp_lines.join("; "));

        assert!(actual.out.contains("Usage"));
    })
}
