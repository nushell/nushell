use nu_test_support::nu;

#[test]
fn rolls_4_roll() {
    let actual = nu!(r#"
        random dice --dice 4 --sides 10 | length
        "#);

    assert_eq!(actual.out, "4");
}
