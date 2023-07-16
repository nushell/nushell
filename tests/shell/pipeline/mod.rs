mod commands;

use nu_test_support::nu;
use pretty_assertions::assert_eq;

#[test]
fn doesnt_break_on_utf8() {
    let actual = nu!("echo ö");

    assert_eq!(actual.out, "ö", "'{}' should contain ö", actual.out);
}
