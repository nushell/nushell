use nu_test_support::{nu, pipeline};

#[test]
fn from_excel_file_to_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.xlsx
            | get SalesOrders
            | get 4
            | get column2
        "#
    ));

    assert_eq!(actual.out, "Gill");
}

#[test]
fn from_excel_file_to_table_select_sheet() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.xlsx --raw
            | from xlsx -s ["SalesOrders"]
            | columns
            | get 0
        "#
    ));

    assert_eq!(actual.out, "SalesOrders");
}
