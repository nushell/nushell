use nu_test_support::nu;

#[test]
fn rolls_4_roll() {
    let actual = nu!(r#"
        random dice -d 4 -s 10 | length
        "#);

    assert_eq!(actual.out, "4");
}
