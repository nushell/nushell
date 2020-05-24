use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn can_only_apply_one() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open caco3_plastics.csv | first 1 | str origin --downcase --upcase"
    );

    assert!(actual.err.contains(r#"--capitalize|--downcase|--upcase|--to-int|--to-float|--substring "start,end"|--replace|--find-replace [pattern replacement]|to-date-time|--trim]"#));
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

        assert_eq!(actual.out, "NUSHELL");
    })
}

#[test]
fn trims() {
    Playground::setup("plugin_str_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu "
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str dependency.name --trim | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "nu");
    })
}

#[test]
fn capitalizes() {
    Playground::setup("plugin_str_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "nu"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str dependency.name --capitalize | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "Nu");
    })
}

#[test]
fn downcases() {
    Playground::setup("plugin_str_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "sample.toml",
            r#"
                    [dependency]
                    name = "LIGHT"
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open sample.toml | str dependency.name -d | get dependency.name | echo $it"
        );

        assert_eq!(actual.out, "light");
    })
}

#[test]
fn upcases() {
    Playground::setup("plugin_str_test_5", |dirs, sandbox| {
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

        assert_eq!(actual.out, "NUSHELL");
    })
}

#[test]
fn converts_to_int() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo '{number_as_string: "1"}'
            | from json
            | str number_as_string --to-int
            | rename number
            | where number == 1
            | get number
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn converts_to_float() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            echo "3.1, 0.0415"
            | split row ","
            | str --to-float
            | sum
        "#
    ));

    assert_eq!(actual.out, "3.1415");
}

#[test]
fn replaces() {
    Playground::setup("plugin_str_test_5", |dirs, sandbox| {
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

        assert_eq!(actual.out, "wykittenshell");
    })
}

#[test]
fn find_and_replaces() {
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
                 | str fortune.teller.phone --find-replace [KATZ "5289"]
                 | get fortune.teller.phone
                 | echo $it
             "#
        ));

        assert_eq!(actual.out, "1-800-5289");
    })
}

#[test]
fn find_and_replaces_without_passing_field() {
    Playground::setup("plugin_str_test_7", |dirs, sandbox| {
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

        assert_eq!(actual.out, "1-800-5289");
    })
}
