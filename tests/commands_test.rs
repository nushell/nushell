mod helpers;

use helpers::in_directory as cwd;

#[test]
fn lines() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip-while $it != \"[dependencies]\" | skip 1 | first 1 | split-column \"=\" | get Column1 | trim | echo $it");

    assert_eq!(output, "rustyline");
}

#[test]
fn open_toml() {
    nu!(output, 
        cwd("tests/fixtures/formats"), 
        "open cargo_sample.toml | get package.edition | echo $it");

    assert_eq!(output, "2018");
}

#[test]
fn open_json() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sgml_description.json | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee | echo $it");

    assert_eq!(output, "markup")
}

#[test]
fn open_xml() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open jonathan.xml | get rss.channel.item.link | echo $it");

    assert_eq!(output, "http://www.jonathanturner.org/2015/10/off-to-new-adventures.html")
}

#[test]
fn open_ini() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open sample.ini | get SectionOne.integer | echo $it");

    assert_eq!(output, "1234")
}

#[test]
fn open_error_if_file_not_found() {
    nu_error!(output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it");

    assert!(output.contains("File cound not be opened"));
}

