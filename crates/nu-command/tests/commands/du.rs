use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::{nu, pipeline, playground::Playground};
use rstest::rstest;

#[test]
fn test_du_flag_min_size() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            du -m -1
        "#
    ));
    assert!(actual
        .err
        .contains("Negative value passed when positive one is required"));

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            du -m 1
        "#
    ));
    assert!(actual.err.is_empty());
}

#[test]
fn test_du_flag_max_depth() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            du -d -2
        "#
    ));
    assert!(actual
        .err
        .contains("Negative value passed when positive one is required"));

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            du -d 2
        "#
    ));
    assert!(actual.err.is_empty());
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn du_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("umv_test_16", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile(src_name)]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            "du -d 1 '{}'",
            src.display(),
        );

        assert!(actual.err.is_empty());
    });
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn du_files_with_glob_metachars_nw(#[case] src_name: &str) {
    du_files_with_glob_metachars(src_name);
}
