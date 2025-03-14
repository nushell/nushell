use nu_test_support::nu;

#[test]
fn const_avg() {
    let actual = nu!("const MODE = [1 3 3 5] | math mode; $MODE");
    assert_eq!(actual.out, "╭───┬───╮│ 0 │ 3 │╰───┴───╯");
}

#[test]
fn can_mode_range_into_list() {
    let actual = nu!("0..5 | math mode");
    let expected = nu!("[0 1 2 3 4 5] | math mode");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_mode_infinite_range() {
    let actual = nu!("0.. | math mode");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
