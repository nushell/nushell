use nu_test_support::nu;

#[test]
fn from_excel_file_to_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data.xlsx
        | get SalesOrders
        | get 4
        | get column2
    "#);

    assert_eq!(actual.out, "Gill");
}

#[test]
fn from_excel_file_to_table_select_sheet() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data.xlsx --raw
        | from xlsx --sheets ["SalesOrders"]
        | columns
        | get 0
    "#);

    assert_eq!(actual.out, "SalesOrders");
}

#[test]
fn from_excel_file_to_date() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data.xlsx
        | get SalesOrders.4.column0
        | format date "%Y-%m-%d"
    "#);

    assert_eq!(actual.out, "2018-02-26");
}
