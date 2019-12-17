use nu_test_support::nu;

#[test]
fn can_get_reverse_first() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | reverse | first 1 | get name | trim | echo $it"
    );

    assert_eq!(actual, "utf16.ini");
}
