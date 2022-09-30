use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu_with_plugins;
use nu_test_support::playground::Playground;

#[test]
fn chooses_highest_increment_if_given_more_than_one() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_inc"),
        "open cargo_sample.toml | first | inc package.version --major --minor | get package.version"
    );

    assert_eq!(actual.out, "1.0.0");

    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_inc"),
        // Regardless of order of arguments
        "open cargo_sample.toml | first | inc package.version --minor --major | get package.version"
    );

    assert_eq!(actual.out, "1.0.0");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | inc package.edition | get package.edition"
        );

        assert_eq!(actual.out, "2019");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | get package.contributors | inc"
        );

        assert_eq!(actual.out, "3");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | inc package.version -M | get package.version"
        );

        assert_eq!(actual.out, "1.0.0");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | inc package.version --minor | get package.version"
        );

        assert_eq!(actual.out, "0.2.0");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | inc package.version --patch | get package.version"
        );

        assert_eq!(actual.out, "0.1.4");
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

        let actual = nu_with_plugins!(
            cwd: dirs.test(),
            plugin: ("nu_plugin_inc"),
            "open sample.toml | get package.version | inc --patch"
        );

        assert_eq!(actual.out, "0.1.4");
    })
}
