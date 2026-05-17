use nu_test_support::prelude::*;

#[test]
fn from_ods_file_to_table() -> Result {
    let code = "
        open sample_data.ods
        | get SalesOrders
        | get 4
        | get column2
    ";
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("Gill")
}

#[test]
fn from_ods_file_to_table_select_sheet() -> Result {
    let code = r#"
        open sample_data.ods --raw
        | from ods --sheets ["SalesOrders"]
        | columns
        | get 0
    "#;
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("SalesOrders")
}

#[test]
fn from_ods_file_to_table_select_sheet_with_annotations() -> Result {
    let code = r#"
        open sample_data_with_annotation.ods --raw
        | from ods --sheets ["SalesOrders"]
        | get SalesOrders
        | get column4
        | get 0
    "#;

    // The Units column in the sheet SalesOrders has an annotation and should be ignored.
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("Units")
}
