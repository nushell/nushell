#[cfg(feature = "plugin")]
use nu_test_support::nu_with_plugins;
use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn which_ls() -> Result {
    test()
        .run("which ls | get type.0")
        .expect_value_eq("built-in")
}

#[test]
fn which_alias_ls() -> Result {
    test()
        .run("alias ls = ls -a; which ls | get path.0 | str trim")
        .expect_value_eq("source")
}

#[test]
fn which_custom_alias() -> Result {
    test()
        .run(r#"alias foo = print "foo!"; which foo | to nuon"#)
        .expect_value_eq(
            r#"[[command, path, type, definition]; [foo, source, alias, "print \"foo!\""]]"#,
        )
}

#[test]
fn which_def_ls() -> Result {
    test()
        .run("def ls [] {echo def}; which ls | get type.0")
        .expect_value_eq("custom")
}

#[test]
fn correct_precedence_alias_def_custom() -> Result {
    // aliases shadow custom commands; `which` only reports the winning
    // declaration. there is no way to invoke the underlying custom command
    // when an alias exists, so returning both entries would be misleading.
    test()
        .run("def ls [] {echo def}; alias ls = echo alias; which ls | get path.0 | str trim")
        .expect_value_eq("source")
}

// `get_aliases_with_name` and `get_custom_commands_with_name` don't return the correct count of
// values
// I suspect this is due to the ScopeFrame getting discarded at '}' and the command is then
// executed in the parent scope
// See: parse_definition, line 2187 for reference.
#[ignore]
#[test]
fn correctly_report_of_shadowed_alias() -> Result {
    let code = "
        alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            which -a xaz
        }
        helper | get path | str contains alias2
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn one_report_of_multiple_defs() -> Result {
    let code = "
        def xaz [] { echo def1 }
        def helper [] {
            def xaz [] { echo def2 }
            which -a xaz
        }
        helper | length
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn def_only_seen_once() -> Result {
    let code = "def xaz [] {echo def1}; which -a xaz | length";
    test().run(code).expect_value_eq(1)
}

#[test]
fn do_not_show_hidden_aliases() -> Result {
    let code = "
        alias foo = echo foo
        hide foo
        which foo | length
    ";

    test().run(code).expect_value_eq(0)
}

#[test]
fn do_not_show_hidden_commands() -> Result {
    let code = "
        def foo [] { echo foo }
        hide foo
        which foo | length
    ";

    test().run(code).expect_value_eq(0)
}

#[test]
fn which_accepts_spread_list() -> Result {
    let code = "
        let apps = [ls];
        $apps | which ...$in | get command.0
    ";

    test().run(code).expect_value_eq("ls")
}

#[test]
fn which_dedup_is_less_than_all() -> Result {
    let all: i32 = test().run("which -a | length")?;
    let dedup: i32 = test().run("which | length")?;

    assert!(all >= dedup);
    Ok(())
}

#[test]
fn which_custom_command_reports_file() -> Result {
    Playground::setup("which_file_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            "
                def foo [] { echo hi }
            ",
        )]);

        let code = "
            source foo.nu
            which foo
        ";

        #[derive(Debug, FromValue)]
        struct Outcome {
            path: String,
        }

        let outcome: (Outcome,) = test().cwd(dirs.test()).run(code)?;
        assert_contains("foo.nu", outcome.0.path);
        Ok(())
    })
}

#[cfg(feature = "plugin")]
#[test]
fn which_plugin_reports_executable() {
    // `example` is the root command provided by nu_plugin_example.
    // `which example` should resolve via plugin_identity to the plugin binary path,
    // which contains "nu_plugin_example" in its filename.
    let actual = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        "which example | to json"
    );

    assert!(
        actual.out.contains("nu_plugin_example"),
        "plugin binary path missing from output: {}",
        actual.out
    );
    assert!(
        actual.out.contains("\"path\""),
        "path column missing from output: {}",
        actual.out
    );
}

#[test]
fn which_external_command_reports_path() -> Result {
    #[derive(Debug, FromValue)]
    struct Outcome {
        path: String,
    }

    // `nu` itself should be on PATH; PATH-found binaries report a non-empty path.
    let outcome: (Outcome,) = test().add_nu_to_path().run("which nu")?;
    // The path value should be non-empty (not just an empty string)
    assert!(!outcome.0.path.is_empty());

    Ok(())
}
