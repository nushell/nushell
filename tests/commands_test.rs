mod helpers;

use h::in_directory as cwd;
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
fn open_error_if_file_not_found() {
    nu_error!(
        output,
        cwd("tests/fixtures/formats"),
        "open i_dont_exist.txt | echo $it"
    );

    assert!(output.contains("File could not be opened"));
}

#[test]
fn save_can_write_out_csv() {
    let (playground_path, tests_dir) = h::setup_playground_for("save_test");

    let full_path     = format!("{}/{}", playground_path, tests_dir         );
    let expected_file = format!("{}/{}", full_path      , "cargo_sample.csv");

    nu!(
        _output,
        cwd(&playground_path),
        "open ../formats/cargo_sample.toml | inc package.version --minor | get package | save save_test/cargo_sample.csv"
    );

    let actual = h::file_contents(&expected_file);
    assert!(actual.contains("[list list],A shell for the GitHub era,2018,ISC,nu,0.2.0"));
}

#[test]
fn cp_can_copy_a_file() {
    let (playground_path, tests_dir) =     h::setup_playground_for("cp_test");

    let full_path     = format!("{}/{}", playground_path, tests_dir         );
    let expected_file = format!("{}/{}", full_path      , "sample.ini"      );

    nu!(
        _output,
        cwd(&playground_path),
        "cp ../formats/sample.ini cp_test/sample.ini"
    );

    assert!(h::file_exists_at(&expected_file));
}

#[test]
fn cp_copies_the_file_inside_directory_if_path_to_copy_is_directory() {
    let (playground_path, tests_dir) =   h::setup_playground_for("cp_test_2");

    let full_path     = format!("{}/{}", playground_path, tests_dir         );
    let expected_file = format!("{}/{}", full_path      , "sample.ini"      );

    nu!(
        _output,
        cwd(&playground_path),
        "cp ../formats/sample.ini cp_test_2"
    );

    assert!(h::file_exists_at(&expected_file));
}

#[test]
fn cp_error_if_attempting_to_copy_a_directory_to_another_directory() {
    let (playground_path, _) = h::setup_playground_for("cp_test_3");

    nu_error!(
        output,
        cwd(&playground_path),
        "cp ../formats cp_test_3"
    );

    assert!(output.contains("../formats"));
    assert!(output.contains("is a directory (not copied)"));
}

#[test]
fn rm_can_remove_a_file() {
    let directory = "tests/fixtures/nuplayground";
    let file = format!("{}/rm_test.txt", directory);

    h::create_file_at(&file);

    nu!(_output, cwd(directory), "rm rm_test.txt");

    assert!(!h::file_exists_at(&file));
}

#[test]
fn rm_can_remove_directory_contents_with_recursive_flag() {
    let (playground_path, tests_dir) = h::setup_playground_for("rm_test");

    for f in ["yehuda.txt", "jonathan.txt", "andres.txt"].iter() {
        h::create_file_at(&format!("{}/{}/{}", playground_path, tests_dir, f));
    }

    nu!(
        _output,
        cwd("tests/fixtures/nuplayground"),
        "rm rm_test --recursive"
    );

    assert!(!h::file_exists_at(&format!("{}/{}", playground_path, tests_dir)));
}

#[test]
fn rm_error_if_attempting_to_delete_a_directory_without_recursive_flag() {
    let (playground_path, tests_dir) = h::setup_playground_for("rm_test_2");
    let full_path = format!("{}/{}", playground_path, tests_dir);

    nu_error!(output, cwd("tests/fixtures/nuplayground"), "rm rm_test_2");

    assert!(h::file_exists_at(&full_path));
    assert!(output.contains("is a directory"));
    h::delete_directory_at(&full_path);
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