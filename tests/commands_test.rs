mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;
use std::path::{Path, PathBuf};

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

#[test]
fn rm_removes_a_file() {
    let sandbox = Playground::setup_for("rm_regular_file_test")
        .with_files(vec![EmptyFile("i_will_be_deleted.txt")])
        .test_dir_name();

    nu!(
        _output,
        cwd(&Playground::root()),
        "rm rm_regular_file_test/i_will_be_deleted.txt"
    );

    let path = &format!(
        "{}/{}/{}",
        Playground::root(),
        sandbox,
        "i_will_be_deleted.txt"
    );

    assert!(!h::file_exists_at(PathBuf::from(path)));
}

#[test]
fn rm_removes_files_with_wildcard() {
    r#" 
    Given these files and directories
        src
        src/cli.rs
        src/lib.rs
        src/prelude.rs
        src/parser
        src/parser/parse.rs
        src/parser/parser.rs
        src/parser/parse
        src/parser/hir
        src/parser/parse/token_tree.rs
        src/parser/hir/baseline_parse.rs
        src/parser/hir/baseline_parse_tokens.rs
    "#;

    let sandbox = Playground::setup_for("rm_wildcard_test")
        .within("src")
        .with_files(vec![
            EmptyFile("cli.rs"),
            EmptyFile("lib.rs"),
            EmptyFile("prelude.rs"),
        ])
        .within("src/parser")
        .with_files(vec![EmptyFile("parse.rs"), EmptyFile("parser.rs")])
        .within("src/parser/parse")
        .with_files(vec![EmptyFile("token_tree.rs")])
        .within("src/parser/hir")
        .with_files(vec![
            EmptyFile("baseline_parse.rs"),
            EmptyFile("baseline_parse_tokens.rs"),
        ])
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);

    r#" The pattern 
            src/*/*/*.rs
        matches
            src/parser/parse/token_tree.rs
            src/parser/hir/baseline_parse.rs
            src/parser/hir/baseline_parse_tokens.rs
    "#;

    nu!(
        _output,
        cwd("tests/fixtures/nuplayground/rm_wildcard_test"),
        "rm \"src/*/*/*.rs\""
    );

    assert!(!h::files_exist_at(
        vec![
            Path::new("src/parser/parse/token_tree.rs"),
            Path::new("src/parser/hir/baseline_parse.rs"),
            Path::new("src/parser/hir/baseline_parse_tokens.rs")
        ],
        PathBuf::from(&full_path)
    ));

    assert_eq!(
        Playground::glob_vec(&format!("{}/src/*/*/*.rs", &full_path)),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn rm_removes_directory_contents_with_recursive_flag() {
    let sandbox = Playground::setup_for("rm_directory_removal_recursively_test")
        .with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ])
        .test_dir_name();

    nu!(
        _output,
        cwd("tests/fixtures/nuplayground"),
        "rm rm_directory_removal_recursively_test --recursive"
    );

    let expected = format!("{}/{}", Playground::root(), sandbox);

    assert!(!h::file_exists_at(PathBuf::from(expected)));
}

#[test]
fn rm_errors_if_attempting_to_delete_a_directory_without_recursive_flag() {
    let sandbox = Playground::setup_for("rm_prevent_directory_removal_without_flag_test").test_dir_name();
    let full_path = format!("{}/{}", Playground::root(), sandbox);

    nu_error!(output, cwd(&Playground::root()), "rm rm_prevent_directory_removal_without_flag_test");

    assert!(h::file_exists_at(PathBuf::from(full_path)));
    assert!(output.contains("is a directory"));
}

#[test]
fn rm_errors_if_attempting_to_delete_single_dot_as_argument() {
    nu_error!(output, cwd(&Playground::root()), "rm .");

    assert!(output.contains("may not be removed"));
}

#[test]
fn rm_errors_if_attempting_to_delete_two_dot_as_argument() {
    nu_error!(output, cwd(&Playground::root()), "rm ..");

    assert!(output.contains("may not be removed"));
}
