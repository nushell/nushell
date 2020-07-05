use nu_test_support::{nu, pipeline};
use std::env;

#[test]
fn locale_correct_en_us() {
    env::set_var("LC_NUMERIC", "en_US.UTF-8");
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 10000.1 | human
        "#
    ));

    assert!(actual.out.contains("10,000.1"));
}

#[test]
fn locale_correct_posix() {
    env::set_var("LC_NUMERIC", "POSIX");
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        = 10000.1 | human
        "#
    ));

    assert!(actual.out.contains("10000.1"));
}
