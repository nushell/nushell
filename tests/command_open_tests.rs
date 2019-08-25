mod helpers;

use helpers::{in_directory as cwd, Playground, Stub::*};

#[test]
fn recognizes_csv() {
    Playground::setup_for("open_recognizes_csv_test").with_files(vec![FileWithContentToBeTrimmed(
        "nu.zion.csv",
        r#"
            author,lang,source
            Jonathan Turner,Rust,New Zealand
            Andres N. Robalino,Rust,Ecuador
            Yehuda Katz,Rust,Estados Unidos
        "#,
    )]);

    nu!(
        output,
        cwd("tests/fixtures/nuplayground/open_recognizes_csv_test"),
        "open nu.zion.csv | where author == \"Andres N. Robalino\" | get source | echo $it"
    );

    assert_eq!(output, "Ecuador");
}

#[test]
fn open_can_parse_bson_1() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.bson | nth 0 | get b | echo $it"
    );

    assert_eq!(output, "hello");
}

#[test]
fn open_can_parse_bson_2() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.bson | nth 6 | get b | get '$binary_subtype' | echo $it "
    );

    assert_eq!(output, "function");
}

#[test]
fn open_can_parse_toml() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | get package.edition | echo $it"
    );

    assert_eq!(output, "2018");
}

#[test]
fn open_can_parse_json() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee | echo $it"
    );

    assert_eq!(output, "markup")
}

#[test]
fn open_can_parse_xml() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open jonathan.xml | get rss.channel.item.link | echo $it"
    );

    assert_eq!(
        output,
        "http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"
    )
}

#[test]
fn open_can_parse_ini() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.ini | get SectionOne.integer | echo $it"
    );

    assert_eq!(output, "1234")
}

#[test]
fn open_can_parse_utf16_ini() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open utf16.ini | get .ShellClassInfo | get IconIndex | echo $it"
    );

    assert_eq!(output, "-236")
}

#[test]
fn errors_if_file_not_found() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it"
    );

    assert!(output.contains("File could not be opened"));
}