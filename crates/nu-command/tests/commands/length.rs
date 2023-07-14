use nu_test_support::nu;

#[test]
fn length_columns_in_cal_table() {
    let actual = nu!("cal | length -c");

    assert_eq!(actual.out, "7");
}

#[test]
fn length_columns_no_rows() {
    let actual = nu!("echo [] | length -c");

    assert_eq!(actual.out, "0");
}
