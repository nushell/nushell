use nu_test_support::nu;

#[test]
fn break_for_loop() {
    let actual = nu!("
        for i in 1..10 { if $i == 2 { break }; print $i }
        ");

    assert_eq!(actual.out, "1");
}

#[test]
fn break_while_loop() {
    let actual = nu!(r#" while true { break }; print "hello" "#);

    assert_eq!(actual.out, "hello");
}
