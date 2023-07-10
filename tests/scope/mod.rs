use nu_test_support::nu;
use pretty_assertions::assert_eq;

#[ignore = "TODO: This shows old-style aliases. New aliases are under commands"]
#[test]
fn scope_shows_alias() {
    let actual = nu!(r#"alias xaz = echo alias1
        scope aliases | find xaz | length
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn scope_shows_command() {
    let actual = nu!(r#"def xaz [] { echo xaz }
        scope commands | find xaz | length
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 1);
}

#[test]
fn scope_doesnt_show_scoped_hidden_alias() {
    let actual = nu!(r#"alias xaz = echo alias1
        do {
            hide xaz
            scope aliases | find xaz | length
        }
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_hidden_alias() {
    let actual = nu!(r#"alias xaz = echo alias1
        hide xaz
        scope aliases | find xaz | length
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_scoped_hidden_command() {
    let actual = nu!(r#"def xaz [] { echo xaz }
        do {
            hide xaz
            scope commands | find xaz | length
        }
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

#[test]
fn scope_doesnt_show_hidden_command() {
    let actual = nu!(r#"def xaz [] { echo xaz }
        hide xaz
        scope commands | find xaz | length
        "#);

    let length: i32 = actual.out.parse().unwrap();
    assert_eq!(length, 0);
}

// same problem as 'which' command
#[ignore]
#[test]
fn correctly_report_of_shadowed_alias() {
    let actual = nu!(r#"alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            scope aliases
        }
        helper | where alias == xaz | get expansion.0"#);

    assert_eq!(actual.out, "echo alias2");
}
