use nu_test_support::{nu, pipeline};

#[test]
fn generates_an_integer() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random integer --min 42 --max 43
        "#
    ));

    assert!(actual.out.contains("42") || actual.out.contains("43"));
}
