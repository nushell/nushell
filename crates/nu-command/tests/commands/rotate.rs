use nu_test_support::{nu, pipeline};

#[test]
fn counter_clockwise() {
    let table = pipeline(
        r#"
        echo [
            [col1, col2, EXPECTED];

            [---, "|||",      XX1]
            [---, "|||",      XX2]
            [---, "|||",      XX3]
        ]
    "#,
    );

    let expected = nu!(pipeline(
        r#"
        echo [
            [  column0, column1, column2, column3];

            [ EXPECTED,    XX1,      XX2,     XX3]
            [     col2,  "|||",    "|||",   "|||"]
            [     col1,    ---,      ---,     ---]
        ]
        | where column0 == EXPECTED
        | get column1 column2 column3
        | str join "-"
        "#,
    ));

    let actual = nu!(format!(
        "{} | {}",
        table,
        pipeline(
            r#"
            rotate --ccw
            | where column0 == EXPECTED
            | get column1 column2 column3
            | str join "-"
        "#
        )
    ));

    assert_eq!(actual.out, expected.out);
}

#[test]
fn clockwise() {
    let table = pipeline(
        r#"
        echo [
            [col1,  col2, EXPECTED];

            [ ---, "|||",      XX1]
            [ ---, "|||",      XX2]
            [ ---, "|||",      XX3]
        ]
    "#,
    );

    let expected = nu!(pipeline(
        r#"
        echo [
            [ column0, column1, column2,  column3];

            [     ---,     ---,     ---,     col1]
            [   "|||",   "|||",   "|||",     col2]
            [     XX3,     XX2,     XX1, EXPECTED]
        ]
        | where column3 == EXPECTED
        | get column0 column1 column2
        | str join "-"
        "#,
    ));

    let actual = nu!(format!(
        "{} | {}",
        table,
        pipeline(
            r#"
            rotate
            | where column3 == EXPECTED
            | get column0 column1 column2
            | str join "-"
        "#
        )
    ));

    assert_eq!(actual.out, expected.out);
}

#[test]
fn different_cols_vals_err() {
    let actual = nu!("[[[one], [two, three]]] | first | rotate");
    assert!(actual
        .err
        .contains("Attempted to create a record from different number of columns and values"))
}
