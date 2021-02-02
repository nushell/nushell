use nu_test_support::nu;

#[test]
fn test_parse_invocation_with_range() {
    let actual = nu!(
        cwd: ".",
        r#"
        let foo = 3
        echo $(echo 1..$foo | each { echo $it }) | to json
        "#
    );
    assert_eq!(actual.out, "[1,2,3]")
}

#[test]
fn create_nothing_in_table() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo [[column]; [$nothing]] | to json
        "#
    );
    assert_eq!(actual.out, "{\"column\":null}");
}

#[test]
fn compare_to_nothing() {
    let actual = nu!(
        cwd: ".",
        r#"
        let f = $nothing
        if $f == $nothing {echo $true} {echo $false}
        "#
    );
    assert_eq!(actual.out, "true");
}
