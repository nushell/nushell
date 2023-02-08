use nu_test_support::nu_with_plugins;

const TEST_CWD: &str = "tests/fixtures/";

#[test]
fn parses_ini() {
    let actual = nu_with_plugins!(
        cwd: TEST_CWD,
        plugin: ("nu_plugin_formats"),
        "open sample.ini | get SectionOne.integer"
    );

    assert_eq!(actual.out, "1234")
}

#[test]
fn parses_utf16_ini() {
    let actual = nu_with_plugins!(
        cwd: TEST_CWD,
        plugin: ("nu_plugin_formats"),
        "open ./utf16.ini --raw | decode utf-16 | from ini | rename info | get info | get IconIndex"
    );

    assert_eq!(actual.out, "-236")
}
