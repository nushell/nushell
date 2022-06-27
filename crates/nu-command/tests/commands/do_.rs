use nu_test_support::{nu, pipeline};

#[test]
fn capture_errors_works() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {$env.use} | get-type
        "#
    ));

    assert_eq!(actual.out, "error");
}
