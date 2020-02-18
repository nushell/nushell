mod commands;

use nu_test_support::nu;

#[test]
fn doesnt_break_on_utf8() {
    let actual = nu!(cwd: ".", "echo ö");

    assert_eq!(actual, "ö", "'{}' should contain ö", actual);
}
