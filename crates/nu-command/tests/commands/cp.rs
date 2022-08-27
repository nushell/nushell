use nu_test_support::fs::file_contents;
use nu_test_support::fs::{files_exist_at, AbsoluteFile, Stub::EmptyFile};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::path::Path;

#[test]
fn copies_a_file() {
    Playground::setup("cp_test_1", |dirs, _| {
        nu!(
            cwd: dirs.root(),
            "cp `{}` cp_test_1/sample.ini",
            dirs.formats().join("sample.ini").display()
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
        let actual = nu!(
            cwd: dirs.formats(),
            "cp ../formats {}", dirs.test().display()
        );

        assert!(actual.err.contains("../formats"));
        assert!(actual.err.contains("resolves to a directory (not copied)"));
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
            "cp originals expected -r"
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
            "cp -r ../formats/* {}", dirs.test().display()
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
            "cp -r * {}", dirs.test().display()
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

#[test]
fn copies_same_file_twice() {
    Playground::setup("cp_test_8", |dirs, _| {
        nu!(
            cwd: dirs.root(),
            "cp `{}` cp_test_8/sample.ini",
            dirs.formats().join("sample.ini").display()
        );

        nu!(
            cwd: dirs.root(),
            "cp `{}` cp_test_8/sample.ini",
            dirs.formats().join("sample.ini").display()
        );

        assert!(dirs.test().join("sample.ini").exists());
    });
}

#[test]
fn copy_files_using_glob_two_parents_up_using_multiple_dots() {
    Playground::setup("cp_test_9", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(vec![
            EmptyFile("jonathan.json"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                cp * ...
            "#
        );

        assert!(files_exist_at(
            vec![
                "yehuda.yaml",
                "jonathan.json",
                "andres.xml",
                "kevin.txt",
                "many_more.ppl",
            ],
            dirs.test()
        ));
    })
}

#[test]
fn copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive() {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("hello_there")]);
        sandbox.mkdir("hello_again");
        sandbox.within("foo").mkdir("bar");

        nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                cp -r .../hello* .
            "#
        );

        let expected = dirs.test().join("foo/bar");

        assert!(files_exist_at(vec!["hello_there", "hello_again"], expected));
    })
}

#[test]
fn copy_to_non_existing_dir() {
    Playground::setup("cp_test_11", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("empty_file")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp empty_file ~/not_a_dir/",
        );
        assert!(actual.err.contains("directory not found"));
        assert!(actual.err.contains("destination directory does not exist"));
    });
}

#[test]
fn copy_dir_contains_symlink_ignored() {
    Playground::setup("cp_test_12", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm tmp_dir/good_bye; cp -r tmp_dir tmp_dir_2",
        );

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` don't exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(vec!["hello_there"], expected.clone()));
        let path = expected.join("dangle_symlink");
        assert!(!path.exists() && !path.is_symlink());
    });
}

#[test]
fn copy_dir_contains_symlink() {
    Playground::setup("cp_test_13", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm tmp_dir/good_bye; cp -r -n tmp_dir tmp_dir_2",
        );

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` also exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(vec!["hello_there"], expected.clone()));
        let path = expected.join("dangle_symlink");
        assert!(path.is_symlink());
    });
}

#[test]
fn copy_dir_symlink_file_body_not_changed() {
    Playground::setup("cp_test_14", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm tmp_dir/good_bye; cp -r -n tmp_dir tmp_dir_2; rm -r tmp_dir; cp -r -n tmp_dir_2 tmp_dir; echo hello_data | save tmp_dir/good_bye",
        );

        // check dangle_symlink in tmp_dir is no longer dangling.
        let expected_file = sandbox.cwd().join("tmp_dir").join("dangle_symlink");
        let actual = file_contents(expected_file);
        assert!(actual.contains("hello_data"));
    });
}

#[test]
fn copy_identical_file() {
    Playground::setup("cp_test_15", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("same.txt")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp same.txt same.txt",
        );
        assert!(actual.err.contains("Copy aborted"));
    });
}

#[test]
fn copy_ignores_ansi() {
    Playground::setup("cp_test_16", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("test.txt")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "ls | find test | get name | cp $in.0 success.txt; ls | find success | get name | ansi strip | get 0",
        );
        assert_eq!(actual.out, "success.txt");
    });
}
