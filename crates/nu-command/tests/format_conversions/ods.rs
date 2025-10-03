use nu_test_support::nu;

#[test]
fn from_ods_file_to_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data.ods
        | get SalesOrders
        | get 4
        | get column2
    "#);

    assert_eq!(actual.out, "Gill");
}

#[test]
fn from_ods_file_to_table_select_sheet() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data.ods --raw
        | from ods --sheets ["SalesOrders"]
        | columns
        | get 0
    "#);

    assert_eq!(actual.out, "SalesOrders");
}

#[test]
fn from_ods_file_to_table_select_sheet_with_annotations() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample_data_with_annotation.ods --raw
        | from ods --sheets ["SalesOrders"]
        | get SalesOrders
        | get column4
        | get 0
    "#);

    // The Units column in the sheet SalesOrders has an annotation and should be ignored.
    assert_eq!(actual.out, "Units");
}
