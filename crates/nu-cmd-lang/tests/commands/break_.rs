use nu_test_support::{nu, pipeline};

#[test]
fn break_for_loop() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        for i in 1..10 { if $i == 2 { break }; print $i }
        "#
    ));

    assert_eq!(actual.out, r#"1"#);
}

#[test]
fn break_while_loop() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        while true { break }; print "hello"
        "#
    ));

    assert_eq!(actual.out, r#"hello"#);
}

#[test]
fn break_each() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        [1, 2, 3, 4, 5] | each {|x| if $x > 3 { break }; $x} | math sum
        "#
    ));

    assert_eq!(actual.out, r#"6"#);
}
