use nu_test_support::{nu, pipeline};

#[test]
fn calculates_two_plus_two() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "2 + 2" | calc
        "#
    ));

    assert!(actual.contains("4.0"));
}

#[test]
fn calculates_two_to_the_power_six() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "2 ^ 6" | calc
        "#
    ));

    assert!(actual.contains("64.0"));
}

#[test]
fn calculates_three_multiplied_by_five() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "3 * 5" | calc
        "#
    ));

    assert!(actual.contains("15.0"));
}

#[test]
fn calculates_twenty_four_divided_by_two() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "24 / 2" | calc
        "#
    ));

    assert!(actual.contains("12.0"));
}
