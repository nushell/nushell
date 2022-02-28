use nu_test_support::{nu, pipeline};

#[test]
fn to_nuon_correct_compaction() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open appveyor.yml 
            | to nuon 
            | str length 
            | $in > 500
        "#
    ));

    assert_eq!(actual.out, "true");
}
