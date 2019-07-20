mod helpers;

use h::in_directory as cwd;
use helpers as h;

#[test]
fn lines() {
    nu!(output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml --raw | lines | skip-while $it != \"[dependencies]\" | skip 1 | first 1 | split-column \"=\" | get Column1 | trim | echo $it");

    assert_eq!(output, "rustyline");
}

#[test]
fn open_csv() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open caco3_plastics.csv | get root | first 1 | get origin | echo $it"
    );

    assert_eq!(output, "SPAIN");
}

#[test]
fn open_toml() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open cargo_sample.toml | get package.edition | echo $it"
    );

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
fn open_ini() {
    nu!(
        output,
        cwd("tests/fixtures/formats"),
        "open sample.ini | get SectionOne.integer | echo $it"
    );

    assert_eq!(output, "1234")
}

#[test]
fn open_error_if_file_not_found() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it"
    );

    assert!(output.contains("File cound not be opened"));
}

#[test]
fn rm() {
    let directory = "tests/fixtures/nuplayground";
    let file = format!("{}/rm_test.txt", directory);

    h::create_file_at(&file);

    nu!(_output, cwd(directory), "rm rm_test.txt");

    assert!(!h::file_exists_at(&file));
}

#[test]
fn can_remove_directory_contents_with_recursive_flag() {
    let path = "tests/fixtures/nuplayground/rm_test";

    if h::file_exists_at(&path) {
        h::delete_directory_at(path)
    }
    h::create_directory_at(path);

    for f in ["yehuda.txt", "jonathan.txt", "andres.txt"].iter() {
        h::create_file_at(&format!("{}/{}", path, f));
    }

    nu!(
        _output,
        cwd("tests/fixtures/nuplayground"),
        "rm rm_test --recursive"
    );

    assert!(!h::file_exists_at(&path));
}

#[test]
fn rm_error_if_attempting_to_delete_a_directory_without_recursive_flag() {
    let path = "tests/fixtures/nuplayground/rm_test_2";

    if h::file_exists_at(&path) {
        h::delete_directory_at(path)
    }
    h::create_directory_at(path);

    nu_error!(output, cwd("tests/fixtures/nuplayground"), "rm rm_test_2");

    assert!(h::file_exists_at(&path));
    assert!(output.contains("is a directory"));
    h::delete_directory_at(path);
}

#[test]
fn rm_error_if_attempting_to_delete_single_dot_as_argument() {
    nu_error!(output, cwd("tests/fixtures/nuplayground"), "rm .");

    assert!(output.contains("may not be removed"));
}

#[test]
fn rm_error_if_attempting_to_delete_two_dot_as_argument() {
    nu_error!(output, cwd("tests/fixtures/nuplayground"), "rm ..");

    assert!(output.contains("may not be removed"));
}
