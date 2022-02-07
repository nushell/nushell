use nu_test_support::{nu, pipeline};

#[test]
fn from_excel_file_to_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.xlsx
            | get SalesOrders
            | nth 4
            | get Column2
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
<<<<<<< HEAD
            | get
=======
            | columns
            | get 0
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    ));

    assert_eq!(actual.out, "SalesOrders");
}
