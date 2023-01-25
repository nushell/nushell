use nu_test_support::{nu, pipeline};

#[test]
fn can_get_reverse_first() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | reverse | first | get name | str trim "
    );

    assert_eq!(actual.out, "utf16.ini");
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!(cwd: ".", pipeline("1 | reverse"));

    assert!(actual.err.contains("only_supports_this_input_type"));
}
