use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error, pipeline};

#[test]
fn can_only_apply_one() {
    let actual = nu_error!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.csv | first 1 | str origin --downcase --upcase"
    );

    assert!(actual.contains(r#"--downcase|--upcase|--to-int|--substring "start,end"|--replace|--find-replace [pattern replacement]]"#));
}

#[test]
fn acts_without_passing_field() {
    Playground::setup("plugin_str_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.yml",
            r#"
                environment:
                  global:
                    PROJECT_NAME: nushell
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.yml | get environment.global.PROJECT_NAME | str --upcase | echo $it"
        );

        assert_eq!(actual, "NUSHELL");
    })
}

#[test]
fn downcases() {
    Playground::setup("plugin_str_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "LIGHT"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str dependency.name --downcase | get dependency.name | echo $it"
        );

        assert_eq!(actual, "light");
    })
}

#[test]
fn upcases() {
    Playground::setup("plugin_str_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [package]
                    name = "nushell"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str package.name --upcase | get package.name | echo $it"
        );

        assert_eq!(actual, "NUSHELL");
    })
}

#[test]
fn converts_to_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open caco3_plastics.csv
            | first 1
            | str tariff_item --to-int
            | where tariff_item == 2509000000
            | get tariff_item
            | echo $it
        "#
    ));

    assert_eq!(actual, "2509000000");
}

#[test]
fn replaces() {
    Playground::setup("plugin_str_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [package]
                     name = "nushell"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str package.name --replace wykittenshell
                 | get package.name
                 | echo $it
             "#
        ));

        assert_eq!(actual, "wykittenshell");
    })
}

#[test]
fn find_and_replaces() {
    Playground::setup("plugin_str_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | str fortune.teller.phone --find-replace [KATZ "5289"]
                 | get fortune.teller.phone
                 | echo $it
             "#
        ));

        assert_eq!(actual, "1-800-5289");
    })
}

#[test]
fn find_and_replaces_without_passing_field() {
    Playground::setup("plugin_str_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                     [fortune.teller]
                     phone = "1-800-KATZ"
                 "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                 open sample.toml
                 | get fortune.teller.phone
                 | str --find-replace [KATZ "5289"]
                 | echo $it
             "#
        ));

        assert_eq!(actual, "1-800-5289");
    })
}
