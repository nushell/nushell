use nu_test_support::{nu, pipeline};

#[test]
fn length_columns_in_cal_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        cal | length -c
        "#
    ));

    assert_eq!(actual.out, "7");
}

#[test]
fn length_columns_no_rows() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo [] | length -c
        "#
    ));

    assert_eq!(actual.out, "0");
}
