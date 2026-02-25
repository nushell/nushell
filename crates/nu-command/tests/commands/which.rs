use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
#[cfg(feature = "plugin")]
use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;

#[test]
fn which_ls() {
    let actual = nu!("which ls | get type.0");

    assert_eq!(actual.out, "built-in");
}

#[test]
fn which_alias_ls() {
    let actual = nu!("alias ls = ls -a; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "source");
}

#[test]
fn which_custom_alias() {
    let actual = nu!(r#"alias foo = print "foo!"; which foo | to nuon"#);

    assert_eq!(
        actual.out,
        r#"[[command, path, type, definition]; [foo, source, alias, "print \"foo!\""]]"#
    );
}

#[test]
fn which_def_ls() {
    let actual = nu!("def ls [] {echo def}; which ls | get type.0");

    assert_eq!(actual.out, "custom");
}

#[test]
fn correct_precedence_alias_def_custom() {
    let actual =
        nu!("def ls [] {echo def}; alias ls = echo alias; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "source");
}

#[test]
fn multiple_reports_for_alias_def_custom() {
    let actual = nu!("def ls [] {echo def}; alias ls = echo alias; which -a ls | length");

    let length: i32 = actual.out.parse().unwrap();
    assert!(length >= 2);
}

// `get_aliases_with_name` and `get_custom_commands_with_name` don't return the correct count of
// values
// I suspect this is due to the ScopeFrame getting discarded at '}' and the command is then
// executed in the parent scope
// See: parse_definition, line 2187 for reference.
#[ignore]
#[test]
fn correctly_report_of_shadowed_alias() {
    let actual = nu!("alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            which -a xaz
        }
        helper | get path | str contains alias2");

    assert_eq!(actual.out, "true");
}

#[test]
fn one_report_of_multiple_defs() {
    let actual = nu!("def xaz [] { echo def1 }
        def helper [] {
            def xaz [] { echo def2 }
            which -a xaz
        }
        helper | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn def_only_seen_once() {
    let actual = nu!("def xaz [] {echo def1}; which -a xaz | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn do_not_show_hidden_aliases() {
    let actual = nu!("alias foo = echo foo
        hide foo
        which foo | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn do_not_show_hidden_commands() {
    let actual = nu!("def foo [] { echo foo }
        hide foo
        which foo | length");

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn which_accepts_spread_list() {
    let actual = nu!(r#"
        let apps = [ls];
        $apps | which ...$in | get command.0
        "#);

    assert_eq!(actual.out, "ls");
}

#[test]
fn which_dedup_is_less_than_all() {
    let all: i32 = nu!("which -a | length").out.parse().unwrap();
    let dedup: i32 = nu!("which | length").out.parse().unwrap();

    assert!(all >= dedup);
}

#[test]
fn which_custom_command_reports_file() {
    Playground::setup("which_file_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                def foo [] { echo hi }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            r#"
        source foo.nu
        which foo | to json
        "#
        );

        // JSON should include an object with a "path" key pointing to foo.nu
        assert!(
            actual.out.contains("\"path\""),
            "output was: {}",
            actual.out
        );
        assert!(
            actual.out.contains("foo.nu"),
            "file value missing: {}",
            actual.out
        );
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
fn which_external_command_reports_path() {
    // `nu` itself should be on PATH; PATH-found binaries report a non-empty path.
    let actual = nu!(r#"which nu | to json"#);
    assert!(
        actual.out.contains("\"path\""),
        "path column missing: {}",
        actual.out
    );
    // The path value should be non-empty (not just an empty string)
    assert!(
        !actual.out.contains("\"path\":\"\""),
        "path was empty: {}",
        actual.out
    );
}
