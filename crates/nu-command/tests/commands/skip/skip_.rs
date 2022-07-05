use nu_test_support::{nu, pipeline};

#[test]
fn binary_skip() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample_data.ods --raw | 
            skip 2 | 
            take 2 | 
            into int
        "#
    ));

    assert_eq!(actual.out, "772");
}
