use nu_test_support::{nu, pipeline};

#[test]
fn int_into_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        10 | into string
        "#
    ));

    assert!(actual.out.eq("10"));
}

#[test]
fn int_into_string_decimals_0() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        10 | into string --decimals 0
        "#
    ));

    assert!(actual.out.eq("10"));
}

#[test]
fn int_into_string_decimals_1() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        10 | into string --decimals 1
        "#
    ));

    assert!(actual.out.eq("10.0"));
}

#[test]
fn int_into_string_decimals_10() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        10 | into string --decimals 10
        "#
    ));

    assert!(actual.out.eq("10.0000000000"));
}

