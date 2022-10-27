use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn row() {
    Playground::setup("merge_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContentToBeTrimmed(
                "caballeros.csv",
                r#"
                name,country,luck
                Andrés,Ecuador,0
                Jonathan,USA,0
                Jason,Canada,0
                Yehuda,USA,0
            "#,
            ),
            FileWithContentToBeTrimmed(
                "new_caballeros.csv",
                r#"
                name,country,luck
                Andrés Robalino,Guayaquil Ecuador,1
                Jonathan Turner,New Zealand,1
            "#,
            ),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open caballeros.csv
                | merge { open new_caballeros.csv }
                | where country in ["Guayaquil Ecuador" "New Zealand"]
                | get luck
                | math sum
                "#
        ));

        assert_eq!(actual.out, "2");
    });
}

#[test]
fn single_record_no_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            {a: 1, b: 5} | merge {c: 2} | to nuon
            "#
        ))
        .out,
        "{a: 1, b: 5, c: 2}"
    );
}

#[test]
fn single_record_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            {a: 1, b: 2} | merge {a: 2} | to nuon
            "#
        ))
        .out,
        "{a: 2, b: 2}"
    );
}

#[test]
fn single_row_table_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            [[a b]; [1 4]] | merge [[a b]; [2 4]] | to nuon
            "#
        ))
        .out,
        "[[a, b]; [2, 4]]"
    );
}

#[test]
fn single_row_table_no_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            [[a b]; [1 4]] | merge [[c d]; [2 4]] | to nuon
            "#
        ))
        .out,
        "[[a, b, c, d]; [1, 4, 2, 4]]"
    );
}

#[test]
fn multi_row_table_no_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            [[a b]; [1 4] [8 9] [9 9]] | merge [[c d]; [2 4]]  | to nuon
            "#
        ))
        .out,
        "[{a: 1, b: 4, c: 2, d: 4}, {a: 8, b: 9}, {a: 9, b: 9}]"
    );
}

#[test]
fn multi_row_table_overwrite() {
    assert_eq!(
        nu!(
            cwd: ".", pipeline(
            r#"
            [[a b]; [1 4] [8 9] [9 9]] | merge  [[a b]; [7 7]]  | to nuon
            "#
        ))
        .out,
        "[[a, b]; [7, 7], [8, 9], [9, 9]]"
    );
}
