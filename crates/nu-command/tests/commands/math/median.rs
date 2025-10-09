use nu_test_support::nu;

#[test]
fn median_numbers_with_even_rows() {
    let actual = nu!(r#"
         echo [10 6 19 21 4]
         | math median
     "#);

    assert_eq!(actual.out, "10")
}

#[test]
fn median_numbers_with_odd_rows() {
    let actual = nu!(r#"
         echo [3 8 9 12 12 15]
         | math median
     "#);

    assert_eq!(actual.out, "10.5")
}

#[test]
fn median_mixed_numbers() {
    let actual = nu!(r#"
         echo [-11.5 -13.5 10]
         | math median
     "#);

    assert_eq!(actual.out, "-11.5")
}

#[test]
fn const_median() {
    let actual = nu!("const MEDIAN = [1 3 5] | math median; $MEDIAN");
    assert_eq!(actual.out, "3");
}

#[test]
fn cannot_median_infinite_range() {
    let actual = nu!("0.. | math median");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
