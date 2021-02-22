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

    let expected = nu!(cwd: ".", pipeline(
        r#"
        echo [
            [  Column0, Column1, Column2, Column3];

            [ EXPECTED,    XX1,      XX2,     XX3]
            [     col2,  "|||",    "|||",   "|||"]
            [     col1,    ---,      ---,     ---]
        ]
        | where Column0 == EXPECTED
        | get Column1 Column2 Column3
        | str collect "-"
        "#,
    ));

    let actual = nu!(
        cwd: ".",
        format!("{} | {}", table, pipeline(r#"
            rotate counter-clockwise
            | where Column0 == EXPECTED
            | get Column1 Column2 Column3
            | str collect "-"
        "#)));

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

    let expected = nu!(cwd: ".", pipeline(
        r#"
        echo [
            [ Column0, Column1, Column2,  Column3];

            [     ---,     ---,     ---,     col1]
            [   "|||",   "|||",   "|||",     col2]
            [     XX3,     XX2,     XX1, EXPECTED]
        ]
        | where Column3 == EXPECTED
        | get Column0 Column1 Column2
        | str collect "-"
        "#,
    ));

    let actual = nu!(
        cwd: ".",
        format!("{} | {}", table, pipeline(r#"
            rotate
            | where Column3 == EXPECTED
            | get Column0 Column1 Column2
            | str collect "-"
        "#)));

    assert_eq!(actual.out, expected.out);
}
