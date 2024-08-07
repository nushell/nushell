use nu_test_support::nu;

#[test]
fn const_avg(){
    let actual = nu!("const MODE = [1 3 3 5] | math mode; $MODE");
    assert_eq!(actual.out, "╭───┬───╮│ 0 │ 3 │╰───┴───╯");
}