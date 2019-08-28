mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn can_only_apply_one() {
    let output = nu_error!(
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | first 1 | inc package.version --major --minor"
    );

    assert!(output.contains("Usage: inc field [--major|--minor|--patch]"));
}

#[test]
fn by_one_with_field_passed() {
    Playground::setup_for("plugin_inc_by_one_with_field_passed_test").with_files(vec![
        FileWithContent(
            "sample.toml",
            r#"
                [package]
                edition = "2018"
            "#,
        ),
    ]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_by_one_with_field_passed_test"),
        "open sample.toml | inc package.edition | get package.edition | echo $it"
    );

    assert_eq!(output, "2019");
}

#[test]
fn by_one_with_no_field_passed() {
    Playground::setup_for("plugin_inc_by_one_with_no_field_passed_test").with_files(vec![
        FileWithContent(
            "sample.toml",
            r#"
                [package]
                contributors = "2"
            "#,
        ),
    ]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_by_one_with_no_field_passed_test"),
        "open sample.toml | get package.contributors | inc | echo $it"
    );

    assert_eq!(output, "3");
}

#[test]
fn semversion_major_inc() {
    Playground::setup_for("plugin_inc_major_semversion_test").with_files(vec![FileWithContent(
        "sample.toml",
        r#"
                [package]
                version = "0.1.3"
            "#,
    )]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_major_semversion_test"),
        "open sample.toml | inc package.version --major | get package.version | echo $it"
    );

    assert_eq!(output, "1.0.0");
}

#[test]
fn semversion_minor_inc() {
    Playground::setup_for("plugin_inc_minor_semversion_test").with_files(vec![FileWithContent(
        "sample.toml",
        r#"
                [package]
                version = "0.1.3"
            "#,
    )]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_minor_semversion_test"),
        "open sample.toml | inc package.version --minor | get package.version | echo $it"
    );

    assert_eq!(output, "0.2.0");
}

#[test]
fn semversion_patch_inc() {
    Playground::setup_for("plugin_inc_patch_semversion_test").with_files(vec![FileWithContent(
        "sample.toml",
        r#"
                [package]
                version = "0.1.3"
            "#,
    )]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_patch_semversion_test"),
        "open sample.toml | inc package.version --patch | get package.version | echo $it"
    );

    assert_eq!(output, "0.1.4");
}

#[test]
fn semversion_without_passing_field() {
    Playground::setup_for("plugin_inc_semversion_without_passing_field_test").with_files(vec![
        FileWithContent(
            "sample.toml",
            r#"
                [package]
                version = "0.1.3"
            "#,
        ),
    ]);

    let output = nu!(
        cwd("tests/fixtures/nuplayground/plugin_inc_semversion_without_passing_field_test"),
        "open sample.toml | get package.version | inc --patch | echo $it"
    );

    assert_eq!(output, "0.1.4");
}
