mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

use std::path::{Path, PathBuf};

#[test]
fn cp_copies_a_file() {
    let sandbox = Playground::setup_for("cp_test").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_file = format!("{}/{}", full_path, "sample.ini");

    nu!(
        _output,
        cwd(&Playground::root()),
        "cp ../formats/sample.ini cp_test/sample.ini"
    );

    assert!(h::file_exists_at(PathBuf::from(expected_file)));
}

#[test]
fn cp_copies_the_file_inside_directory_if_path_to_copy_is_directory() {
    let sandbox = Playground::setup_for("cp_test_2").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_file = format!("{}/{}", full_path, "sample.ini");

    nu!(
        _output,
        cwd(&Playground::root()),
        "cp ../formats/sample.ini cp_test_2"
    );

    assert!(h::file_exists_at(PathBuf::from(expected_file)));
}

#[test]
fn cp_error_if_attempting_to_copy_a_directory_to_another_directory() {
    Playground::setup_for("cp_test_3");

    nu_error!(output, cwd(&Playground::root()), "cp ../formats cp_test_3");

    assert!(output.contains("../formats"));
    assert!(output.contains("is a directory (not copied)"));
}

#[test]
fn cp_copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag() {
    let sandbox = Playground::setup_for("cp_test_4")
        .within("originals")
        .with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ])
        .within("copies_expected")
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_dir = format!("{}/{}", full_path, "copies_expected/originals");

    nu!(
        _output,
        cwd(&full_path),
        "cp originals copies_expected --recursive"
    );

    assert!(h::dir_exists_at(PathBuf::from(&expected_dir)));
    assert!(h::files_exist_at(vec![Path::new("yehuda.txt"), Path::new("jonathan.txt"), Path::new("andres.txt")],  PathBuf::from(&expected_dir)));
}