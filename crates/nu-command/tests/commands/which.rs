use nu_test_support::nu;

#[test]
fn which_ls() {
    let actual = nu!(
        cwd: ".",
        "which ls | get path.0 | str trim"
    );

    assert_eq!(actual.out, "Nushell built-in command");
}

#[test]
fn which_alias_ls() {
    let actual = nu!(
        cwd: ".",
        "alias ls = ls -a; which ls | get path.0 | str trim"
    );

    assert_eq!(actual.out, "Nushell alias: ls -a");
}

#[test]
fn which_def_ls() {
    let actual = nu!(
        cwd: ".",
        "def ls [] {echo def}; which ls | get path.0 | str trim"
    );

    assert_eq!(actual.out, "Nushell custom command");
}

#[test]
fn correct_precedence_alias_def_custom() {
    let actual = nu!(
        cwd: ".",
        "def ls [] {echo def}; alias ls = echo alias; which ls | get path.0 | str trim"
    );

    assert_eq!(actual.out, "Nushell alias: echo alias");
}

#[test]
fn multiple_reports_for_alias_def_custom() {
    let actual = nu!(
        cwd: ".",
        "def ls [] {echo def}; alias ls = echo alias; which -a ls | length"
    );

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
    let actual = nu!(
        cwd: ".",
        r#"alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            which -a xaz
        }
        helper | get path | str contains alias2"#
    );

    assert_eq!(actual.out, "true");
}

#[test]
fn one_report_of_multiple_defs() {
    let actual = nu!(
        cwd: ".",
        r#"def xaz [] { echo def1 }
        def helper [] {
            def xaz [] { echo def2 }
            which -a xaz
        }
        helper | length"#
    );

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn def_only_seen_once() {
    let actual = nu!(
        cwd: ".",
        "def xaz [] {echo def1}; which -a xaz | length"
    );

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn do_not_show_hidden_aliases() {
    let actual = nu!(
        cwd: ".",
        r#"alias foo = echo foo
        hide foo
        which foo | length"#
    );

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn do_not_show_hidden_commands() {
    let actual = nu!(
        cwd: ".",
        r#"def foo [] { echo foo }
        hide foo
        which foo | length"#
    );

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}
