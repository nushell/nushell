use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error};

#[test]
fn can_only_apply_one() {
    let actual = nu_error!(
        cwd: "tests/fixtures/formats",
        "open cargo_sample.toml | first 1 | inc package.version --major --minor"
    );

    assert!(actual.contains("Usage: inc field [--major|--minor|--patch]"));
}

#[test]
fn by_one_with_field_passed() {
    Playground::setup("plugin_inc_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    edition = "2018"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | inc package.edition | get package.edition | echo $it"
        );

        assert_eq!(actual, "2019");
    })
}

#[test]
fn by_one_with_no_field_passed() {
    Playground::setup("plugin_inc_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    contributors = "2"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | get package.contributors | inc | echo $it"
        );

        assert_eq!(actual, "3");
    })
}

#[test]
fn semversion_major_inc() {
    Playground::setup("plugin_inc_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    version = "0.1.3"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | inc package.version --major | get package.version | echo $it"
        );

        assert_eq!(actual, "1.0.0");
    })
}

#[test]
fn semversion_minor_inc() {
    Playground::setup("plugin_inc_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    version = "0.1.3"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | inc package.version --minor | get package.version | echo $it"
        );

        assert_eq!(actual, "0.2.0");
    })
}

#[test]
fn semversion_patch_inc() {
    Playground::setup("plugin_inc_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    version = "0.1.3"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | inc package.version --patch | get package.version | echo $it"
        );

        assert_eq!(actual, "0.1.4");
    })
}

#[test]
fn semversion_without_passing_field() {
    Playground::setup("plugin_inc_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    version = "0.1.3"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | get package.version | inc --patch | echo $it"
        );

        assert_eq!(actual, "0.1.4");
    })
}
