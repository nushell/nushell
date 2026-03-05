use nu_test_support::prelude::*;

#[test]
fn from_ods_file_to_table() -> Result {
    let code = r#"
        open sample_data.ods
        | get SalesOrders
        | get 4
        | get column2
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "Gill");
    Ok(())
}

#[test]
fn from_ods_file_to_table_select_sheet() -> Result {
    let code = r#"
        open sample_data.ods --raw
        | from ods --sheets ["SalesOrders"]
        | columns
        | get 0
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "SalesOrders");
    Ok(())
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
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "Units");
    Ok(())
}
