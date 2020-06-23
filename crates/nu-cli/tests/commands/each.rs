use nu_test_support::{nu, pipeline};

#[test]
fn each_works_separately() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        echo [1 2 3] | each { echo $it 10 | math sum } | to json | echo $it
        "#
    ));

    assert_eq!(actual.out, "[11,12,13]");
}
