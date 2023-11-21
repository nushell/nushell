use nu_test_support::{nu, pipeline};

#[test]
fn columns() {
    let actual = nu!(pipeline(
        "
            echo [
              [arepas, color];
              [3,  white]
              [8, yellow]
              [4,  white]
            ] | drop column | columns | length
        "
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn drop_columns_positive_value() {
    let actual = nu!("echo [[a, b];[1,2]] | drop column -1");

    assert!(actual.err.contains("use a positive value"));
}

#[test]
fn more_columns_than_table_has() {
    let actual = nu!(pipeline(
        "
            echo [
              [arepas, color];
              [3,  white]
              [8, yellow]
              [4,  white]
            ] | drop column 3 | columns | is-empty
        "
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn rows() {
    let actual = nu!(pipeline(
        "
            echo [
              [arepas];

              [3]
              [8]
              [4]
            ]
            | drop 2
            | get arepas
            | math sum
        "
    ));

    assert_eq!(actual.out, "3");
}

#[test]
fn more_rows_than_table_has() {
    let actual = nu!("[date] | drop 50 | length");

    assert_eq!(actual.out, "0");
}

#[test]
fn nth_range_inclusive() {
    let actual = nu!("echo 10..15 | drop nth (2..3) | to json --raw");

    assert_eq!(actual.out, "[10,11,14,15]");
}

#[test]
fn nth_range_exclusive() {
    let actual = nu!("echo 10..15 | drop nth (1..<3) | to json --raw");

    assert_eq!(actual.out, "[10,13,14,15]");
}

#[test]
fn nth_missing_first_argument() {
    let actual = nu!("echo 10..15 | drop nth \"\"");

    assert!(actual.err.contains("int or range"));
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | drop 50");

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn disjoint_columns_first_row_becomes_empty() {
    let actual = nu!(pipeline(
        "
            [{a: 1}, {b: 2, c: 3}]
            | drop column
            | columns
            | to nuon
        "
    ));

    assert_eq!(actual.out, "[b, c]");
}

#[test]
fn disjoint_columns() {
    let actual = nu!(pipeline(
        "
            [{a: 1, b: 2}, {c: 3}]
            | drop column
            | columns
            | to nuon
        "
    ));

    assert_eq!(actual.out, "[a, c]");
}

#[test]
fn record() {
    let actual = nu!("{a: 1, b: 2} | drop column | to nuon");

    assert_eq!(actual.out, "{a: 1}");
}

#[test]
fn more_columns_than_record_has() {
    let actual = nu!("{a: 1, b: 2} | drop column 3 | to nuon");

    assert_eq!(actual.out, "{}");
}
