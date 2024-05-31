use nu_test_support::nu;

#[test]
fn grid_errors_with_few_columns() {
    let actual = nu!("[1 2 3 4 5] | grid --width 5");

    assert!(actual.err.contains("Couldn't fit grid into 5 columns"));
}
