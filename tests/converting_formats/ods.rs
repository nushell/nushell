use nu_test_support::{nu, pipeline};

#[test]
fn from_ods_file_to_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.ods
            | get SalesOrders
            | nth 4
            | get Column2
            | echo $it
        "#
    ));

    assert_eq!(actual, "Gill");
}
