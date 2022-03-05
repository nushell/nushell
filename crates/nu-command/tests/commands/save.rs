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
    Playground::setup("save_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("cargo_sample.csv");

        nu!(
            cwd: dirs.root(),
            r#"echo [[name, version, description, license, edition]; [nu, "0.14", "A new type of shell", "MIT", "2018"]] | save save_test_2/cargo_sample.csv"#,
        );

        let actual = file_contents(expected_file);
        println!("{}", actual);
        assert!(actual.contains("nu,0.14,A new type of shell,MIT,2018"));
    })
}

#[test]
fn writes_out_list() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("list_sample.txt");

        nu!(
            cwd: dirs.root(),
            r#"echo [a b c d] | save save_test_3/list_sample.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "a\nb\nc\nd\n")
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn save_append_will_create_file_if_not_exists() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![]);

        let expected_file = dirs.test().join("new-file.txt");

        nu!(
            cwd: dirs.root(),
            r#"echo hello | save --raw --append save_test_3/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{}", actual);
        assert!(actual == "hello");
    })
}
