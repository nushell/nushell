use nu_test_support::prelude::*;

#[test]
fn const_avg() -> Result {
    let expected = Value::test_list(vec![Value::test_int(3)]);
    test()
        .run("const MODE = [1 3 3 5] | math mode; $MODE")
        .expect_value_eq(expected)
}

#[test]
fn mode_table_cell_path_gives_column_modes() -> Result {
    // The mode of a column is a list, so the record holds a list per column.
    test()
        .run("[[a b]; [1 x] [2 y] [2 z]] | math mode a | to nuon")
        .expect_value_eq("{a: [2]}")
}

#[test]
fn cannot_mode_range() -> Result {
    let outcome = test().run("0..5 | math mode").expect_parse_error()?;

    assert!(matches!(outcome, ParseError::InputMismatch { .. }));
    Ok(())
}
