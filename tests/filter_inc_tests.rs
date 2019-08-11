mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn can_only_apply_one() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | first 1 | inc package.version --major --minor"
    );

    assert!(output.contains("Usage: inc field [--major|--minor|--patch]"));
}

#[test]
fn regular_field_by_one() {
    Playground::setup_for("plugin_inc_test_1")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                edition = "2018"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_1"),
        "open sample.toml | inc package.edition | get package.edition | echo $it"
    );

    assert_eq!(output, "2019");
}


#[test]
fn by_one_without_passing_field() {
    Playground::setup_for("plugin_inc_test_2")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                contributors = "2"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_2"),
        "open sample.toml | get package.contributors | inc | echo $it"
    );

    assert_eq!(output, "3");
}

#[test]
fn semversion_major_inc() {
    Playground::setup_for("plugin_inc_test_3")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                version = "0.1.3"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_3"),
        "open sample.toml | inc package.version --major | get package.version | echo $it"
    );

    assert_eq!(output, "1.0.0");
}

#[test]
fn semversion_minor_inc() {
    Playground::setup_for("plugin_inc_test_4")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                version = "0.1.3"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_4"),
        "open sample.toml | inc package.version --minor | get package.version | echo $it"
    );

    assert_eq!(output, "0.2.0");
}

#[test]
fn semversion_patch_inc() {
    Playground::setup_for("plugin_inc_test_5")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                version = "0.1.3"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_5"),
        "open sample.toml | inc package.version --patch | get package.version | echo $it"
    );

    assert_eq!(output, "0.1.4");
}

#[test]
fn semversion_without_passing_field() {
    Playground::setup_for("plugin_inc_test_6")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                version = "0.1.3"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_inc_test_6"),
        "open sample.toml | get package.version | inc --patch | echo $it"
    );

    assert_eq!(output, "0.1.4");
}