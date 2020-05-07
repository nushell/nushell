use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn semicolon_allows_lhs_to_complete() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(),
            "touch i_will_be_created_semi.txt; echo done"
        );

        let path = dirs.test().join("i_will_be_created_semi.txt");

        assert!(path.exists());
        assert_eq!(actual.out, "done");
    })
}

#[test]
fn semicolon_lhs_error_stops_processing() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
    r#"
        where 1 1; echo done
    "#
    ));

    assert!(!actual.out.contains("done"));
}
