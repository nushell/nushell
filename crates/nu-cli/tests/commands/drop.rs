use nu_test_support::nu;

#[test]
fn drop_rows() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"echo '[{"foo": 3}, {"foo": 8}, {"foo": 4}]' | from json | drop 2 | get foo | sum | echo $it"#
    );

    assert_eq!(actual.out, "3");
}
