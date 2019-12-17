use nu_test_support::fs::{files_exist_at, AbsoluteFile, Stub::EmptyFile};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_error};
use std::path::Path;

#[test]
fn copies_a_file() {
    Playground::setup("cp_test_1", |dirs, _| {
        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_1/sample.ini",
            dirs.formats().join("sample.ini")
        );

        assert!(dirs.test().join("sample.ini").exists());
    });
}

#[test]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory() {
    Playground::setup("cp_test_2", |dirs, _| {
        let expected_file = AbsoluteFile::new(dirs.test().join("sample.ini"));

        nu!(
            cwd: dirs.formats(),
            "cp ../formats/sample.ini {}",
            expected_file.dir()
        );

        assert!(dirs.test().join("sample.ini").exists());
    })
}

#[test]
fn error_if_attempting_to_copy_a_directory_to_another_directory() {
    Playground::setup("cp_test_3", |dirs, _| {
        let actual = nu_error!(
            cwd: dirs.formats(),
            "cp ../formats {}", dirs.test()
        );

        assert!(actual.contains("../formats"));
        assert!(actual.contains("is a directory (not copied)"));
    });
}

#[test]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag() {
    Playground::setup("cp_test_4", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jonathan.txt"),
                EmptyFile("andres.txt"),
            ])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");

        nu!(
            cwd: dirs.test(),
            "cp originals expected --recursive"
        );

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            vec![
                Path::new("yehuda.txt"),
                Path::new("jonathan.txt"),
                Path::new("andres.txt")
            ],
            expected_dir
        ));
    })
}

#[test]
fn deep_copies_with_recursive_flag() {
    Playground::setup("cp_test_5", |dirs, sandbox| {
        sandbox
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

        nu!(
            cwd: dirs.test(),
            "cp originals expected --recursive"
        );

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            vec![Path::new("errors.txt"), Path::new("multishells.txt")],
            jonathans_expected_copied_dir
        ));
        assert!(files_exist_at(
            vec![Path::new("coverage.txt"), Path::new("commands.txt")],
            andres_expected_copied_dir
        ));
        assert!(files_exist_at(
            vec![Path::new("defer-evaluation.txt")],
            yehudas_expected_copied_dir
        ));
    })
}

#[test]
fn copies_using_path_with_wildcard() {
    Playground::setup("cp_test_6", |dirs, _| {
        nu!(
            cwd: dirs.formats(),
            "cp ../formats/* {}", dirs.test()
        );

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
        nu!(
            cwd: dirs.formats(),
            "cp * {}", dirs.test()
        );

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
