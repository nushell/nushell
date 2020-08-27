use nu_test_support::{nu, pipeline};

#[test]
fn evaluates_two_plus_two() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        math eval "2 + 2"
        "#
    ));

    assert!(actual.out.contains("4.0"));
}

#[test]
fn evaluates_two_to_the_power_four() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "2 ^ 4" | math eval
        "#
    ));

    assert!(actual.out.contains("16.0"));
}

#[test]
fn evaluates_three_multiplied_by_five() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "3 * 5" | math eval
        "#
    ));

    assert!(actual.out.contains("15.0"));
}

#[test]
fn evaluates_twenty_four_divided_by_two() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "24 / 2" | math eval
        "#
    ));

    assert!(actual.out.contains("12.0"));
}

#[test]
fn evaluates_twenty_eight_minus_seven() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "28 - 7" | math eval
        "#
    ));

    assert!(actual.out.contains("21"));
}

#[test]
fn evaluates_pi() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        math eval pi
        "#
    ));

    assert!(actual.out.contains("3.14"));
}
