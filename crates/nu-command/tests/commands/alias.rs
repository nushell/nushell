use nu_test_support::{nu, pipeline};

#[test]
fn alias_simple() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        alias bar = source sample_def.nu; bar; greet
        "#
    ));

    assert_eq!(actual.out, "hello");
}
