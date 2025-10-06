use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::{nu, playground::Playground};
use rstest::rstest;

#[test]
fn test_du_flag_min_size() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        du -m -1
    "#);
    assert!(
        actual
            .err
            .contains("Negative value passed when positive one is required")
    );

    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        du -m 1
    "#);
    assert!(actual.err.is_empty());
}

#[test]
fn test_du_flag_max_depth() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        du -d -2
    "#);
    assert!(
        actual
            .err
            .contains("Negative value passed when positive one is required")
    );

    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        du -d 2
    "#);
    assert!(actual.err.is_empty());
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn du_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("du_test_16", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(src_name)]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "du -d 1 '{}'",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());

        // also test for variables.
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "let f = '{}'; du -d 1 $f",
                src.display(),
            )
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

#[test]
fn du_with_multiple_path() {
    let actual = nu!(cwd: "tests/fixtures", "du cp formats | get path | path basename");
    assert!(actual.out.contains("cp"));
    assert!(actual.out.contains("formats"));
    assert!(!actual.out.contains("lsp"));
    assert!(actual.status.success());

    // report errors if one path not exists
    let actual = nu!(cwd: "tests/fixtures", "du cp asdf | get path | path basename");
    assert!(actual.err.contains("nu::shell::io::not_found"));
    assert!(!actual.status.success());

    // du with spreading empty list should returns nothing.
    let actual = nu!(cwd: "tests/fixtures", "du ...[] | length");
    assert_eq!(actual.out, "0");
}

#[test]
fn test_du_output_columns() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "du -m 1 | columns | str join ','"
    );
    assert_eq!(actual.out, "path,apparent,physical");
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "du -m 1 -l | columns | str join ','"
    );
    assert_eq!(actual.out, "path,apparent,physical,directories,files");
}
