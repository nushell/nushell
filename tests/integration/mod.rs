use nu_test_support::nu;
use pretty_assertions::assert_str_eq;

#[test]
fn multiword_commands_have_their_parent_commands() {
    let out = nu!(r#"
        scope commands
        | where type == built-in and name like ' '
        | where ($it.name | split row ' ' | first) not-in (
            scope commands
            | where type in [keyword built-in]
            | get name
        )
        | get name
        | to json --raw
    "#);

    assert_str_eq!(
        "[]",
        out.out,
        "These multiword commands are missing their dummy parent commands: {}",
        out.out
    );
}
