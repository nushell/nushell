mod helpers;

use h::{in_directory as cwd, Playground, Stub::*};
use helpers as h;

use std::path::{Path, PathBuf};

#[test]
fn copies_a_file() {
    let sandbox = Playground::setup_for("cp_test_1").test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_file = format!("{}/{}", full_path, "sample.ini");

    nu!(
        _output,
        cwd(&Playground::root()),
        "cp ../formats/sample.ini cp_test_1/sample.ini"
    );

    assert!(h::file_exists_at(PathBuf::from(expected_file)));
}

#[test]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory() {
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
fn error_if_attempting_to_copy_a_directory_to_another_directory() {
    Playground::setup_for("cp_test_3");

    nu_error!(output, cwd(&Playground::root()), "cp ../formats cp_test_3");

    assert!(output.contains("../formats"));
    assert!(output.contains("is a directory (not copied)"));
}

#[test]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag() {
    let sandbox = Playground::setup_for("cp_test_4")
        .within("originals")
        .with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ])
        .mkdir("copies_expected")
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_dir = format!("{}/{}", full_path, "copies_expected/originals");

    nu!(
        _output,
        cwd(&full_path),
        "cp originals copies_expected --recursive"
    );

    assert!(h::dir_exists_at(PathBuf::from(&expected_dir)));
    assert!(h::files_exist_at(
        vec![
            Path::new("yehuda.txt"),
            Path::new("jonathan.txt"),
            Path::new("andres.txt")
        ],
        PathBuf::from(&expected_dir)
    ));
}

#[test]
fn deep_copies_with_recursive_flag() {
    r#" 
    Given these files and directories
        originals
        originals/manifest.txt
        originals/contributors
        originals/contributors/yehuda.txt
        originals/contributors/jonathan.txt
        originals/contributors/andres.txt
        originals/contributors/jonathan
        originals/contributors/jonathan/errors.txt
        originals/contributors/jonathan/multishells.txt
        originals/contributors/andres
        originals/contributors/andres/coverage.txt
        originals/contributors/andres/commands.txt
        originals/contributors/yehuda
        originals/contributors/yehuda/defer-evaluation.txt
    "#;

    let sandbox = Playground::setup_for("cp_test_5")
        .within("originals")
        .with_files(vec![EmptyFile("manifest.txt")])
        .within("originals/contributors")
        .with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ])
        .within("originals/contributors/jonathan")
        .with_files(vec![EmptyFile("errors.txt"), EmptyFile("multishells.txt")])
        .within("originals/contributors/andres")
        .with_files(vec![EmptyFile("coverage.txt"), EmptyFile("commands.txt")])
        .within("originals/contributors/yehuda")
        .with_files(vec![EmptyFile("defer-evaluation.txt")])
        .mkdir("copies_expected")
        .test_dir_name();

    let full_path = format!("{}/{}", Playground::root(), sandbox);
    let expected_dir = format!("{}/{}", full_path, "copies_expected/originals");

    let jonathans_expected_copied_dir = format!("{}/contributors/jonathan", expected_dir);
    let andres_expected_copied_dir = format!("{}/contributors/andres", expected_dir);
    let yehudas_expected_copied_dir = format!("{}/contributors/yehuda", expected_dir);

    nu!(
        _output,
        cwd(&full_path),
        "cp originals copies_expected --recursive"
    );

    assert!(h::dir_exists_at(PathBuf::from(&expected_dir)));
    assert!(h::files_exist_at(
        vec![Path::new("errors.txt"), Path::new("multishells.txt")],
        PathBuf::from(&jonathans_expected_copied_dir)
    ));
    assert!(h::files_exist_at(
        vec![Path::new("coverage.txt"), Path::new("commands.txt")],
        PathBuf::from(&andres_expected_copied_dir)
    ));
    assert!(h::files_exist_at(
        vec![Path::new("defer-evaluation.txt")],
        PathBuf::from(&yehudas_expected_copied_dir)
    ));
}

#[test]
fn copies_using_path_with_wildcard() {
    let sandbox = Playground::setup_for("cp_test_6").test_dir_name();
    let expected_copies_path = format!("{}/{}", Playground::root(), sandbox);

    nu!(
        _output,
        cwd(&Playground::root()),
        "cp ../formats/* cp_test_6"
    );

    assert!(h::files_exist_at(
        vec![
            Path::new("caco3_plastics.csv"),
            Path::new("cargo_sample.toml"),
            Path::new("jonathan.xml"),
            Path::new("sample.ini"),
            Path::new("sgml_description.json"),
            Path::new("utf16.ini"),
        ],
        PathBuf::from(&expected_copies_path)
    ));
}

#[test]
fn copies_using_a_glob() {
    let sandbox = Playground::setup_for("cp_test_7").test_dir_name();
    let expected_copies_path = format!("{}/{}", Playground::root(), sandbox);

    nu!(
        _output,
        cwd("tests/fixtures/formats"),
        "cp * ../nuplayground/cp_test_7"
    );

    assert!(h::files_exist_at(
        vec![
            Path::new("caco3_plastics.csv"),
            Path::new("cargo_sample.toml"),
            Path::new("jonathan.xml"),
            Path::new("sample.ini"),
            Path::new("sgml_description.json"),
            Path::new("utf16.ini"),
        ],
        PathBuf::from(&expected_copies_path)
    ));
}
