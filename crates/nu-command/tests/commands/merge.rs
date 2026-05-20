use nu_protocol::{test_record, test_table};
use nu_test_support::prelude::*;

#[test]
fn row() -> Result {
    let mut tester = test();

    let left_sample = test_table![
        ["name", "country", "luck"];
        ["Andrés", "Ecuador", 0],
        ["JT", "USA", 0],
        ["Jason", "Canada", 0],
        ["Yehuda", "USA", 0]
    ];
    let right_sample = test_table![
        ["name", "country", "luck"];
        ["Andrés Robalino", "Guayaquil Ecuador", 1],
        ["JT Turner", "New Zealand", 1]
    ];

    let () = tester.run_with_data("let left = $in", left_sample)?;
    let () = tester.run_with_data("let right = $in", right_sample)?;

    let code = r#"
        $left
        | merge $right
        | where country in ["Guayaquil Ecuador" "New Zealand"]
        | get luck
        | math sum
    "#;

    tester.run(code).expect_value_eq(2)
}

#[test]
fn single_record_no_overwrite() -> Result {
    test()
        .run("{a: 1, b: 5} | merge {c: 2}")
        .expect_value_eq(test_record! { "a" => 1, "b" => 5, "c" => 2 })
}

#[test]
fn single_record_overwrite() -> Result {
    test()
        .run("{a: 1, b: 2} | merge {a: 2}")
        .expect_value_eq(test_record! {"a" => 2, "b" => 2})
}

#[test]
fn single_row_table_overwrite() -> Result {
    test()
        .run("[[a b]; [1 4]] | merge [[a b]; [2 4]]")
        .expect_value_eq(test_table![["a", "b"]; [2, 4]])
}

#[test]
fn single_row_table_no_overwrite() -> Result {
    test()
        .run("[[a b]; [1 4]] | merge [[c d]; [2 4]]")
        .expect_value_eq(test_table! [["a", "b", "c", "d"]; [1, 4, 2, 4]])
}

#[test]
fn multi_row_table_no_overwrite() -> Result {
    test()
        .run("[[a b]; [1 4] [8 9] [9 9]] | merge [[c d]; [2 4]]")
        .expect_value_eq([
            test_record! {
              "a" => 1,
              "b" => 4,
              "c" => 2,
              "d" => 4
            },
            test_record! {
              "a" => 8,
              "b" => 9
            },
            test_record! {
              "a" => 9,
              "b" => 9
            },
        ])
}

#[test]
fn multi_row_table_overwrite() -> Result {
    test()
        .run("[[a b]; [1 4] [8 9] [9 9]] | merge  [[a b]; [7 7]]")
        .expect_value_eq(test_table![
            ["a", "b"];
            [7, 7],
            [8, 9],
            [9, 9],
        ])
}
