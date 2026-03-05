use nu_test_support::prelude::*;

#[test]
fn from_excel_file_to_table() -> Result {
    let code = r#"
        open sample_data.xlsx
        | get SalesOrders
        | get 4
        | get column2
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "Gill");
    Ok(())
}

#[test]
fn from_excel_file_to_table_select_sheet() -> Result {
    let code = r#"
        open sample_data.xlsx --raw
        | from xlsx --sheets ["SalesOrders"]
        | columns
        | get 0
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "SalesOrders");
    Ok(())
}

#[test]
fn from_excel_file_to_date() -> Result {
    let code = r#"
        open sample_data.xlsx
        | get SalesOrders.4.column0
        | format date "%Y-%m-%d"
    "#;
    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "2018-02-26");
    Ok(())
}
