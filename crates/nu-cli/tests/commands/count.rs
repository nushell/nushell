use nu_test_support::{nu, pipeline};

#[test]
fn count_columns_in_cal_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal | count -c
        "#
    ));

    assert_eq!(actual.out, "7");
}

#[test]
fn count_columns_no_rows() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [] | count -c
        "#
    ));

    assert_eq!(actual.out, "0");
}
