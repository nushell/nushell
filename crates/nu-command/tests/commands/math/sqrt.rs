use nu_test_support::nu;

#[test]
fn can_sqrt_numbers() {
    let actual = nu!(
        cwd: ".",
        "echo [0.25 2 4] | math sqrt | math sum"
    );

    assert_eq!(actual.out, "3.914213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573");
}

#[test]
fn can_sqrt_irrational() {
    let actual = nu!(
        cwd: ".",
        "echo 2 | math sqrt"
    );

    assert_eq!(actual.out, "1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573");
}

#[test]
fn can_sqrt_perfect_square() {
    let actual = nu!(
        cwd: ".",
        "echo 4 | math sqrt"
    );

    assert_eq!(actual.out, "2");
}
