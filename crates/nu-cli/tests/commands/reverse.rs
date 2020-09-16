use nu_test_support::nu;

#[test]
fn can_get_reverse_first() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | reverse | first 1 | get name | str trim | echo $it"
    );

    assert_eq!(actual.out, "utf16.ini");
}
