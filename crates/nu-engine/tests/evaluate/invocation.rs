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
