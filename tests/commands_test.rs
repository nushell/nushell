mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

#[test]
fn lines() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip-while $it != \"[dependencies]\" | skip 1 | first 1 | split-column \"=\" | get Column1 | trim | echo $it"
    );

    assert_eq!(output, "rustyline");
}

#[test]
fn open_can_parse_csv() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | first 1 | get origin | echo $it"
    );

    assert_eq!(output, "SPAIN");
}

#[test]
fn open_can_parse_bson_1() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.bson | nth 3 | get b | get '$javascript' | echo $it"
    );

    assert_eq!(h::normalize_string(&output), "\"let x = y\"");
}

#[test]
fn open_can_parse_bson_2() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.bson | nth 0 | get b | echo $it"
    );

    assert_eq!(output, "hello");
}

#[test]
fn open_can_parse_bson_3() {
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
fn open_error_if_file_not_found() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it"
    );

    assert!(output.contains("File could not be opened"));
}

#[test]
fn save_figures_out_intelligently_where_to_write_out_with_metadata() {
    let sandbox = Playground::setup_for("save_smart_test")
        .with_files(vec![FileWithContent(
            "cargo_sample.toml",
            r#"
                [package]
                name = "nu"
                version = "0.1.1"
                authors = ["Yehuda Katz <wycats@gmail.com>"]
                description = "A shell for the GitHub era"
                license = "ISC"
                edition = "2018"
            "#,
        )])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let subject_file = format!("{}/{}", full_path, "cargo_sample.toml");

    nu!(
        _output,
        cwd(&Playground::root()),
        "open save_smart_test/cargo_sample.toml | inc package.version --minor | save"
    );

    let actual = h::file_contents(&subject_file);
    assert!(actual.contains("0.2.0"));
}

#[test]
fn save_can_write_out_csv() {
    let sandbox = Playground::setup_for("save_writes_out_csv_test").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_file = format!("{}/{}", full_path, "cargo_sample.csv");

    nu!(
        _output,
        cwd(&Playground::root()),
        "open ../formats/cargo_sample.toml | inc package.version --minor | get package | save save_writes_out_csv_test/cargo_sample.csv"
    );

    let actual = h::file_contents(&expected_file);
    assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
}
