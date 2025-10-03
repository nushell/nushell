use nu_test_support::fs::Stub::{EmptyFile, FileWithContentToBeTrimmed};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn creates_the_resulting_string_from_the_given_fields() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open cargo_sample.toml
        | get package
        | format pattern "{name} has license {license}"
    "#);

    assert_eq!(actual.out, "nu has license ISC");
}

#[test]
fn format_input_record_output_string() {
    let actual = nu!(r#"{name: Downloads} | format pattern "{name}""#);

    assert_eq!(actual.out, "Downloads");
}

#[test]
fn given_fields_can_be_column_paths() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open cargo_sample.toml
        | format pattern "{package.name} is {package.description}"
    "#);

    assert_eq!(actual.out, "nu is a new type of shell");
}

#[test]
fn cant_use_variables() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open cargo_sample.toml
        | format pattern "{$it.package.name} is {$it.package.description}"
    "#);

    // TODO SPAN: This has been removed during SpanId refactor
    assert!(actual.err.contains("Removed functionality"));
}

#[test]
fn error_unmatched_brace() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
    open cargo_sample.toml
        | format pattern "{package.name"
    "#);

    assert!(actual.err.contains("unmatched curly brace"));
}

#[test]
fn format_filesize_works() {
    Playground::setup("format_filesize_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | format filesize kB size
            | get size
            | first
        ");

        assert_eq!(actual.out, "0 kB");
    })
}

#[test]
fn format_filesize_works_with_nonempty_files() {
    Playground::setup(
        "format_filesize_works_with_nonempty_files",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
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
            assert_eq!(actual.out, "25 B");

            #[cfg(windows)]
            assert_eq!(actual.out, "27 B");
        },
    )
}

#[test]
fn format_filesize_with_invalid_unit() {
    let actual = nu!("1MB | format filesize sec");

    assert!(actual.err.contains("invalid_unit"));
}

#[test]
fn format_duration_with_invalid_unit() {
    let actual = nu!("1sec | format duration MB");

    assert!(actual.err.contains("invalid_unit"));
}
