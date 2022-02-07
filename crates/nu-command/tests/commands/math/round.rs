use nu_test_support::nu;

#[test]
fn can_round_very_large_numbers() {
    let actual = nu!(
        cwd: ".",
        "echo 18.1372544780074142289927665486772012345 | math round"
    );

    assert_eq!(actual.out, "18")
}

#[test]
fn can_round_very_large_numbers_with_precision() {
    let actual = nu!(
        cwd: ".",
        "echo 18.13725447800741422899276654867720121457878988 | math round -p 10"
    );

    assert_eq!(actual.out, "18.137254478")
}
