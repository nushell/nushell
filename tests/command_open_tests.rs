mod helpers;

use helpers as h;
use helpers::{Playground, Stub::*};

#[test]
fn recognizes_csv() {
    Playground::setup("open_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu.zion.csv",
            r#"
                    author,lang,source
                    Jonathan Turner,Rust,New Zealand
                    Andres N. Robalino,Rust,Ecuador
                    Yehuda Katz,Rust,Estados Unidos
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), h::pipeline(
            r#"
                open nu.zion.csv
                | where author == "Andres N. Robalino"
                | get source
                | echo $it
            "#
        ));

        assert_eq!(actual, "Ecuador");
    })
}

#[test]
fn open_can_parse_bson_1() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open sample.bson | get root | nth 0 | get b | echo $it"
    );

    assert_eq!(actual, "hello");
}

#[test]
fn open_can_parse_bson_2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sample.bson
            | get root
            | nth 6
            | get b
            | get '$binary_subtype'
            | echo $it
        "#
    ));

    assert_eq!(actual, "function");
}

#[test]
fn open_can_parse_sqlite() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sample.db
            | get table_values
            | nth 2
            | get x
            | echo $it"#
    ));

    assert_eq!(actual, "hello");
}

#[test]
fn open_can_parse_toml() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open cargo_sample.toml | get package.edition | echo $it"
    );

    assert_eq!(actual, "2018");
}

#[test]
fn open_can_parse_tsv() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open caco3_plastics.tsv
            | first 1
            | get origin
            | echo $it
        "#
    ));

    assert_eq!(actual, "SPAIN")
}

#[test]
fn open_can_parse_json() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
        r#"
            open sgml_description.json
            | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
            | echo $it
        "#
    ));

    assert_eq!(actual, "markup")
}

#[test]
fn open_can_parse_xml() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open jonathan.xml | get rss.channel.item.link | echo $it"
    );

    assert_eq!(
        actual,
        "http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"
    )
}

#[test]
fn open_can_parse_ini() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open sample.ini | get SectionOne.integer | echo $it"
    );

    assert_eq!(actual, "1234")
}

#[test]
fn open_can_parse_utf16_ini() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open utf16.ini | get .ShellClassInfo | get IconIndex | echo $it"
    );

    assert_eq!(actual, "-236")
}

#[test]
fn errors_if_file_not_found() {
    let actual = nu_error!(
        cwd: "tests/fixtures/formats",
        "open i_dont_exist.txt | echo $it"
    );

    assert!(actual.contains("File could not be opened"));
}
