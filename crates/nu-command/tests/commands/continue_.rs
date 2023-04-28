use nu_test_support::nu;

#[test]
fn continue_for_loop() {
    let actual = nu!("for i in 1..10 { if $i == 2 { continue }; print $i }");

    assert_eq!(actual.out, "1345678910");
}
