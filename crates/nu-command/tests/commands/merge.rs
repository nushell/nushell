use nu_test_support::nu;

#[test]
fn row() {
    let left_sample = r#"[[name, country, luck];
        [Andrés, Ecuador, 0],
        [JT, USA, 0],
        [Jason, Canada, 0],
        [Yehuda, USA, 0]]"#;

    let right_sample = r#"[[name, country, luck];
         ["Andrés Robalino", "Guayaquil Ecuador", 1],
         ["JT Turner", "New Zealand", 1]]"#;

    let actual = nu!(format!(
        r#"
            ({left_sample})
            | merge ({right_sample})
            | where country in ["Guayaquil Ecuador" "New Zealand"]
            | get luck
            | math sum
        "#
    ));

    assert_eq!(actual.out, "2");
}

#[test]
fn single_record_no_overwrite() {
    assert_eq!(
        nu!("
            {a: 1, b: 5} | merge {c: 2} | to nuon
            ")
        .out,
        "{a: 1, b: 5, c: 2}"
    );
}

#[test]
fn single_record_overwrite() {
    assert_eq!(
        nu!("
            {a: 1, b: 2} | merge {a: 2} | to nuon
            ")
        .out,
        "{a: 2, b: 2}"
    );
}

#[test]
fn single_row_table_overwrite() {
    assert_eq!(
        nu!("
            [[a b]; [1 4]] | merge [[a b]; [2 4]] | to nuon
            ")
        .out,
        "[[a, b]; [2, 4]]"
    );
}

#[test]
fn single_row_table_no_overwrite() {
    assert_eq!(
        nu!("
            [[a b]; [1 4]] | merge [[c d]; [2 4]] | to nuon
            ")
        .out,
        "[[a, b, c, d]; [1, 4, 2, 4]]"
    );
}

#[test]
fn multi_row_table_no_overwrite() {
    assert_eq!(
        nu!("
            [[a b]; [1 4] [8 9] [9 9]] | merge [[c d]; [2 4]]  | to nuon
            ")
        .out,
        "[{a: 1, b: 4, c: 2, d: 4}, {a: 8, b: 9}, {a: 9, b: 9}]"
    );
}

#[test]
fn multi_row_table_overwrite() {
    assert_eq!(
        nu!("
            [[a b]; [1 4] [8 9] [9 9]] | merge  [[a b]; [7 7]]  | to nuon
            ")
        .out,
        "[[a, b]; [7, 7], [8, 9], [9, 9]]"
    );
}
