use nu_test_support::{nu, pipeline};

#[test]
fn row() {
    let actual = nu!(
    cwd: ".", pipeline(
    r#"
        [[key value]; [foo 1] [foo 2]] | transpose -r | debug
            "#
    ));

    assert_eq!(actual.out.matches("foo").collect::<Vec<&str>>(), ["foo"]);
}
