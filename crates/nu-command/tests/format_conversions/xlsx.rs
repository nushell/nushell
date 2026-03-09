use nu_test_support::prelude::*;

#[test]
fn from_excel_file_to_table() -> Result {
    let code = r#"
        open sample_data.xlsx
        | get SalesOrders
        | get 4
        | get column2
    "#;
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("Gill")
}

#[test]
fn from_excel_file_to_table_select_sheet() -> Result {
    let code = r#"
        open sample_data.xlsx --raw
        | from xlsx --sheets ["SalesOrders"]
        | columns
        | get 0
    "#;
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("SalesOrders")
}

#[test]
fn from_excel_file_to_date() -> Result {
    let code = r#"
        open sample_data.xlsx
        | get SalesOrders.4.column0
        | format date "%Y-%m-%d"
    "#;
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("2018-02-26")
}
