use nu_test_support::nu;

#[test]
fn can_sqrt_numbers() {
    let actual = nu!(
        cwd: ".",
        "echo [0.25 2 4] | math sqrt | math sum"
    );

    assert_eq!(actual.out, "3.914213562373095");
}

#[test]
fn can_sqrt_irrational() {
    let actual = nu!(
        cwd: ".",
        "echo 2 | math sqrt"
    );

    assert_eq!(actual.out, "1.4142135623730951");
}

#[test]
fn can_sqrt_perfect_square() {
    let actual = nu!(
        cwd: ".",
        "echo 4 | math sqrt"
    );

    assert_eq!(actual.out, "2");
}
