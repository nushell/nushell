use nu_test_support::{nu, pipeline};

#[test]
fn rolls_4_roll() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random dice -d 4 -s 10 | count
        "#
    ));

    assert_eq!(actual.out, "4");
}
