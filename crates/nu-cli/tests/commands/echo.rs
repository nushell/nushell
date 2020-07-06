use nu_test_support::{nu, pipeline};

#[test]
fn echo_range_is_lazy() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo 1..10000000000 | first 3 | echo $it | to json
        "#
    ));

    assert_eq!(actual.out, "[1,2,3]");
}
