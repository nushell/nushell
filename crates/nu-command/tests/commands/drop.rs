use nu_test_support::{nu, pipeline};

#[test]
fn columns() {
    let actual = nu!(
        cwd: ".", pipeline(r#"
            echo [
              [arepas, color];

              [3,  white]
              [8, yellow]
              [4,  white]
            ]
            | drop column
            | get
            | count
        "#)
    );

    assert_eq!(actual.out, "1");
}

#[test]
fn more_columns_than_table_has() {
    let actual = nu!(
        cwd: ".", pipeline(r#"
            echo [
              [arepas, color];

              [3,  white]
              [8, yellow]
              [4,  white]
            ]
            | drop column 3
            | get
            | empty?
        "#)
    );

    assert_eq!(actual.out, "true");
}

#[test]
fn rows() {
    let actual = nu!(
        cwd: ".", pipeline(r#"
            echo [
              [arepas];

              [3]
              [8]
              [4]
            ]
            | drop 2
            | get arepas
            | math sum
        "#)
    );

    assert_eq!(actual.out, "3");
}

#[test]
fn more_rows_than_table_has() {
    let actual = nu!(cwd: ".", "date | drop 50 | count");

    assert_eq!(actual.out, "0");
}
