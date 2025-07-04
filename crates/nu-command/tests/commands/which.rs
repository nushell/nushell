use nu_test_support::nu;

#[test]
fn which_ls() {
    let actual = nu!("which ls | get type.0");

    assert_eq!(actual.out, "built-in");
}

#[ignore = "TODO: Can't have alias recursion"]
#[test]
fn which_alias_ls() {
    let actual = nu!("alias ls = ls -a; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "Nushell alias: ls -a");
}

#[test]
fn which_custom_alias() {
    let actual = nu!(r#"alias foo = print "foo!"; which foo | to nuon"#);

    assert_eq!(actual.out, r#"[[command, path, type]; [foo, "", alias]]"#);
}

#[test]
fn which_def_ls() {
    let actual = nu!("def ls [] {echo def}; which ls | get type.0");

    assert_eq!(actual.out, "custom");
}

#[ignore = "TODO: Can't have alias with the same name as command"]
#[test]
fn correct_precedence_alias_def_custom() {
    let actual =
        nu!("def ls [] {echo def}; alias ls = echo alias; which ls | get path.0 | str trim");

    assert_eq!(actual.out, "Nushell alias: echo alias");
}

#[ignore = "TODO: Can't have alias with the same name as command"]
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
    let actual = nu!(
        cwd: ".",  // or any valid path
        r#"
        let apps = [ls]; 
        $apps | which ...$in | get command.0
        "#
    );

    assert_eq!(actual.out, "ls");
}
