use nu_test_support::prelude::*;

#[test]
fn columns_flag_updates_only_the_listed_columns() -> Result {
    test()
        .run("{a: 1, b: 2, c: 3} | update cells -c [a] { $in + 10 } | to nuon")
        .expect_value_eq("{a: 11, b: 2, c: 3}")
}

#[test]
fn empty_columns_flag_updates_nothing() -> Result {
    test()
        .run("{a: 1, b: 2, c: 3} | update cells -c [] { $in + 10 } | to nuon")
        .expect_value_eq("{a: 1, b: 2, c: 3}")
}

#[test]
fn columns_flag_and_rest_columns_are_merged() -> Result {
    test()
        .run("{a: 1, b: 2, c: 3} | update cells -c [a] { $in + 10 } c | to nuon")
        .expect_value_eq("{a: 11, b: 2, c: 13}")
}

#[test]
fn empty_column_spread_updates_nothing() -> Result {
    let code = r#"
        let table = [[name price]; [apple 1] [pear 2]]
        let columns_to_double = $table | columns | where $it == "weight"
        $table | update cells { $in * 2 } ...$columns_to_double | to nuon
    "#;

    test()
        .run(code)
        .expect_value_eq("[[name, price]; [apple, 1], [pear, 2]]")
}

#[test]
fn int_column_names_in_spread_match_columns_flag() -> Result {
    let code = r#"
        let columns_to_update = [1]
        {"1": 1, b: 2} | update cells { $in + 1 } ...$columns_to_update | to nuon
    "#;

    test().run(code).expect_value_eq(r#"{"1": 2, b: 2}"#)
}

#[test]
fn int_column_names_in_columns_flag() -> Result {
    test()
        .run(r#"{"1": 1, b: 2} | update cells -c [1] { $in + 1 } | to nuon"#)
        .expect_value_eq(r#"{"1": 2, b: 2}"#)
}
