mod commands;

use nu_test_support::nu;
use pretty_assertions::assert_eq;

#[test]
fn doesnt_break_on_utf8() {
    let actual = nu!("echo ö");
    assert_eq!(actual.out, "ö", "'{}' should contain ö", actual.out);
}

#[test]
fn non_zero_exit_code_in_middle_of_pipeline_ignored() {
    let actual = nu!("nu -c 'print a b; exit 42' | collect");
    assert_eq!(actual.out, "ab");

    let actual = nu!("nu -c 'print a b; exit 42' | nu --stdin -c 'collect'");
    assert_eq!(actual.out, "ab");
}

#[test]
fn infinite_output_piped_to_value() {
    let actual = nu!("nu --testbin iecho x | 1");
    assert_eq!(actual.out, "1");
    assert_eq!(actual.err, "");
}
