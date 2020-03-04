use nu_test_support::fs::{file_contents, Stub::FileWithContent};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn figures_out_intelligently_where_to_write_out_with_metadata() {
    Playground::setup("save_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "cargo_sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.1.1"
                authors = ["Yehuda Katz <wycats@gmail.com>"]
                description = "A shell for the GitHub era"
                license = "ISC"
                edition = "2018"
            "#,
        )]);

        let subject_file = dirs.test().join("cargo_sample.toml");

        nu!(
            cwd: dirs.root(),
            "open save_test_1/cargo_sample.toml | save"
        );

        let actual = file_contents(&subject_file);
        assert!(actual.contains("0.1.1"));
    })
}

#[test]
fn writes_out_csv() {
    Playground::setup("save_test_2", |dirs, _| {
        let expected_file = dirs.test().join("cargo_sample.csv");

        nu!(
            cwd: dirs.root(),
            "open {}/cargo_sample.toml | get package | save save_test_2/cargo_sample.csv",
            dirs.formats()
        );

        let actual = file_contents(expected_file);
        assert!(actual.contains("[Table],A shell for the GitHub era,2018,ISC,nu,0.1.1"));
    })
}
