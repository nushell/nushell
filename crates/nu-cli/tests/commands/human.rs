use nu_test_support::{nu, pipeline};
use std::env;

#[test]
fn calculates_two_plus_two() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "2 + 2" | calc
        "#
    ));

    assert!(actual.out.contains("4.0"));
}

#[test]
fn locale_correct_en_us() {
    assert!(env::set_var("LC_NUMERIC", "en_US").is_ok());

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 10000.1
        "#
    ));

    assert!(actual.out.contains("10,000.1"));
}

#[test]
fn locale_correct_posix() {
    assert!(env::set_var("LC_NUMERIC", "POSIX").is_ok());

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 10000.1
        "#
    ));

    assert!(actual.out.contains("10000.1"));
}
