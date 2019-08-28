mod helpers;

use helpers::{dir_exists_at, file_exists_at, files_exist_at, Playground, Stub::*};

use nu::AbsoluteFile;
use std::path::{Path, PathBuf};

#[test]
fn copies_a_file() {
    Playground::setup("cp_test_1", |dirs, _| {
        nu!(
            dirs.root(),
            "cp {} cp_test_1/sample.ini",
            dirs.formats().join("sample.ini")
        );

        assert!(file_exists_at(dirs.test().join("sample.ini")));
    });
}

#[test]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory() {
    Playground::setup("cp_test_2", |dirs, _| {
        let expected_file = AbsoluteFile::new(dirs.test().join("sample.ini"));

        nu!(
            dirs.formats(),
            "cp ../formats/sample.ini {}",
            expected_file.dir()
        );

        assert!(file_exists_at(dirs.test().join("sample.ini")));
    })
}

#[test]
fn error_if_attempting_to_copy_a_directory_to_another_directory() {
    Playground::setup("cp_test_3", |dirs, _| {
        let output = nu_error!(dirs.formats(), "cp ../formats {}", dirs.test());

        assert!(output.contains("../formats"));
        assert!(output.contains("is a directory (not copied)"));
    });
}

#[test]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag() {
    Playground::setup("cp_test_4", |dirs, playground| {
        playground
            .within("originals")
            .with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jonathan.txt"),
                EmptyFile("andres.txt"),
            ])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");

        nu!(dirs.test(), "cp originals expected --recursive");

        assert!(dir_exists_at(PathBuf::from(&expected_dir)));
        assert!(files_exist_at(
            vec![
                Path::new("yehuda.txt"),
                Path::new("jonathan.txt"),
                Path::new("andres.txt")
            ],
            PathBuf::from(&expected_dir)
        ));
    })
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

    Playground::setup("cp_test_5", |dirs, playground| {
        playground
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
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");

        let jonathans_expected_copied_dir = expected_dir.join("contributors").join("jonathan");
        let andres_expected_copied_dir = expected_dir.join("contributors").join("andres");
        let yehudas_expected_copied_dir = expected_dir.join("contributors").join("yehuda");

        nu!(dirs.test(), "cp originals expected --recursive");

        assert!(dir_exists_at(PathBuf::from(&expected_dir)));
        assert!(files_exist_at(
            vec![Path::new("errors.txt"), Path::new("multishells.txt")],
            PathBuf::from(&jonathans_expected_copied_dir)
        ));
        assert!(files_exist_at(
            vec![Path::new("coverage.txt"), Path::new("commands.txt")],
            PathBuf::from(&andres_expected_copied_dir)
        ));
        assert!(files_exist_at(
            vec![Path::new("defer-evaluation.txt")],
            PathBuf::from(&yehudas_expected_copied_dir)
        ));
    })
}

#[test]
fn copies_using_path_with_wildcard() {
    Playground::setup("cp_test_6", |dirs, _| {
        nu!(dirs.formats(), "cp ../formats/* {}", dirs.test());

        assert!(files_exist_at(
            vec![
                Path::new("caco3_plastics.csv"),
                Path::new("cargo_sample.toml"),
                Path::new("jonathan.xml"),
                Path::new("sample.ini"),
                Path::new("sgml_description.json"),
                Path::new("utf16.ini"),
            ],
            dirs.test()
        ));
    })
}

#[test]
fn copies_using_a_glob() {
    Playground::setup("cp_test_7", |dirs, _| {
        nu!(dirs.formats(), "cp * {}", dirs.test());

        assert!(files_exist_at(
            vec![
                Path::new("caco3_plastics.csv"),
                Path::new("cargo_sample.toml"),
                Path::new("jonathan.xml"),
                Path::new("sample.ini"),
                Path::new("sgml_description.json"),
                Path::new("utf16.ini"),
            ],
            dirs.test()
        ));
    });
}
