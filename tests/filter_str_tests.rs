mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn can_only_apply_one() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | str origin --downcase --upcase"
    );

    assert!(output.contains("Usage: str field [--downcase|--upcase|--to-int]"));
}

#[test]
fn acts_without_passing_field() {
    Playground::setup_for("plugin_str_test_without_passing_field")
        .with_files(vec![FileWithContent(
            "sample.yml",
            r#"
                environment:
                  global:
                    PROJECT_NAME: nushell
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_str_test_without_passing_field"),
        "open sample.yml | get environment.global.PROJECT_NAME | str --upcase | echo $it"
    );

    assert_eq!(output, "NUSHELL");
}

#[test]
fn downcases() {
    Playground::setup_for("plugin_str_test_downcases")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [dependency]
                name = "LIGHT"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_str_test_downcases"),
        "open sample.toml | str dependency.name --downcase | get dependency.name | echo $it"
    );

    assert_eq!(output, "light");
}

#[test]
fn upcases() {
    Playground::setup_for("plugin_str_test_upcases")
        .with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                [package]
                name = "nushell"
            "#,
        )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/plugin_str_test_upcases"),
        "open sample.toml | str package.name --upcase | get package.name | echo $it"
    );

    assert_eq!(output, "NUSHELL");
}

#[test]
fn converts_to_int() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | str tariff_item --to-int | where tariff_item == 2509000000 | get tariff_item | echo $it"
    );

    assert_eq!(output, "2509000000");
}