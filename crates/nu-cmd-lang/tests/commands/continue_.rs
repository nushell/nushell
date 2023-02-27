use nu_test_support::{nu, pipeline};

#[test]
fn continue_for_loop() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        for i in 1..10 { if $i == 2 { continue }; print $i }
        "#
    ));

    assert_eq!(actual.out, r#"1345678910"#);
}
