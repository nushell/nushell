use nu_test_support::fs::Stub::{EmptyFile, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn creates_the_resulting_string_from_the_given_fields() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | get package
            | format "{name} has license {license}"
        "#
    ));

    assert_eq!(actual.out, "nu has license ISC");
}

#[test]
fn given_fields_can_be_column_paths() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | format "{package.name} is {package.description}"
        "#
    ));

    assert_eq!(actual.out, "nu is a new type of shell");
}

#[test]
fn can_use_variables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        open cargo_sample.toml
            | format "{$it.package.name} is {$it.package.description}"
        "#
    ));

    assert_eq!(actual.out, "nu is a new type of shell");
}

#[test]
fn format_filesize_works() {
    Playground::setup("format_filesize_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | format filesize KB size
                | get size
                | first
            "#
        ));

        assert_eq!(actual.out, "0.0 KB");
    })
}

#[test]
fn format_filesize_works_with_nonempty_files() {
    Playground::setup(
        "format_filesize_works_with_nonempty_files",
        |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContentToBeTrimmed(
                "sample.toml",
                r#"
                    [dependency]
                    name = "nu"
                "#,
            )]);

            let actual = nu!(
                cwd: dirs.test(),
                "ls sample.toml | format filesize B size | get size | first"
            );

            #[cfg(not(windows))]
            assert_eq!(actual.out, "25");

            #[cfg(windows)]
            assert_eq!(actual.out, "27");
        },
    )
}
