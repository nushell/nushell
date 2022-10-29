use nu_test_support::fs::{files_exist_at, Stub::EmptyFile};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn moves_a_file() {
    Playground::setup("mv_test_1", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("andres.txt")])
            .mkdir("expected");

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("expected/yehuda.txt");

        nu!(
            cwd: dirs.test(),
            "mv andres.txt expected/yehuda.txt"
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn overwrites_if_moving_to_existing_file_and_force_provided() {
    Playground::setup("mv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("andres.txt"), EmptyFile("jonathan.txt")]);

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jonathan.txt");

        nu!(
            cwd: dirs.test(),
            "mv andres.txt -f jonathan.txt"
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_a_directory() {
    Playground::setup("mv_test_3", |dirs, sandbox| {
        sandbox.mkdir("empty_dir");

        let original_dir = dirs.test().join("empty_dir");
        let expected = dirs.test().join("renamed_dir");

        nu!(
            cwd: dirs.test(),
            "mv empty_dir renamed_dir"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_4", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("jonathan.txt");
        let expected = dirs.test().join("expected/jonathan.txt");

        nu!(
            cwd: dirs.test(),
            "mv jonathan.txt expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("mv_test_5", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(vec![EmptyFile("jonathan.txt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        nu!(
            cwd: dirs.test(),
            "mv contributors expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
        assert!(files_exist_at(vec!["jonathan.txt"], expected))
    })
}

#[test]
fn moves_using_path_with_wildcard() {
    Playground::setup("mv_test_7", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(vec![
                EmptyFile("andres.ini"),
                EmptyFile("caco3_plastics.csv"),
                EmptyFile("cargo_sample.toml"),
                EmptyFile("jonathan.ini"),
                EmptyFile("jonathan.xml"),
                EmptyFile("sgml_description.json"),
                EmptyFile("sample.ini"),
                EmptyFile("utf16.ini"),
                EmptyFile("yehuda.ini"),
            ])
            .mkdir("work_dir")
            .mkdir("expected");

        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(cwd: work_dir, "mv ../originals/*.ini ../expected");

        assert!(files_exist_at(
            vec!["yehuda.ini", "jonathan.ini", "sample.ini", "andres.ini",],
            expected
        ));
    })
}

#[test]
fn moves_using_a_glob() {
    Playground::setup("mv_test_8", |dirs, sandbox| {
        sandbox
            .within("meals")
            .with_files(vec![
                EmptyFile("arepa.txt"),
                EmptyFile("empanada.txt"),
                EmptyFile("taquiza.txt"),
            ])
            .mkdir("work_dir")
            .mkdir("expected");

        let meal_dir = dirs.test().join("meals");
        let work_dir = dirs.test().join("work_dir");
        let expected = dirs.test().join("expected");

        nu!(cwd: work_dir, "mv ../meals/* ../expected");

        assert!(meal_dir.exists());
        assert!(files_exist_at(
            vec!["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
    })
}

#[test]
fn moves_a_directory_with_files() {
    Playground::setup("mv_test_9", |dirs, sandbox| {
        sandbox
            .mkdir("vehicles/car")
            .mkdir("vehicles/bicycle")
            .with_files(vec![
                EmptyFile("vehicles/car/car1.txt"),
                EmptyFile("vehicles/car/car2.txt"),
            ])
            .with_files(vec![
                EmptyFile("vehicles/bicycle/bicycle1.txt"),
                EmptyFile("vehicles/bicycle/bicycle2.txt"),
            ]);

        let original_dir = dirs.test().join("vehicles");
        let expected_dir = dirs.test().join("expected");

        nu!(
            cwd: dirs.test(),
            "mv vehicles expected"
        );

        assert!(!original_dir.exists());
        assert!(expected_dir.exists());
        assert!(files_exist_at(
            vec![
                "car/car1.txt",
                "car/car2.txt",
                "bicycle/bicycle1.txt",
                "bicycle/bicycle2.txt"
            ],
            expected_dir
        ));
    })
}

#[test]
fn errors_if_source_doesnt_exist() {
    Playground::setup("mv_test_10", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let actual = nu!(
            cwd: dirs.test(),
            "mv non-existing-file test_folder/"
        );
        assert!(actual.err.contains("invalid file or pattern"));
    })
}

#[test]
fn error_if_moving_to_existing_file_without_force() {
    Playground::setup("mv_test_10_0", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("andres.txt"), EmptyFile("jonathan.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv andres.txt jonathan.txt"
        );
        assert!(actual.err.contains("file already exists"))
    })
}

#[test]
fn errors_if_destination_doesnt_exist() {
    Playground::setup("mv_test_10_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("empty.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv empty.txt does/not/exist"
        );

        assert!(actual.err.contains("directory not found"));
    })
}

#[test]
fn errors_if_multiple_sources_but_destination_not_a_directory() {
    Playground::setup("mv_test_10_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("file1.txt"),
            EmptyFile("file2.txt"),
            EmptyFile("file3.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv file?.txt not_a_dir"
        );

        assert!(actual
            .err
            .contains("Can only move multiple sources if destination is a directory"));
    })
}

#[test]
fn errors_if_renaming_directory_to_an_existing_file() {
    Playground::setup("mv_test_10_3", |dirs, sandbox| {
        sandbox
            .mkdir("mydir")
            .with_files(vec![EmptyFile("empty.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv mydir empty.txt"
        );

        assert!(actual.err.contains("Can't move a directory"),);
        assert!(actual.err.contains("to a file"),);
    })
}

#[test]
fn errors_if_moving_to_itself() {
    Playground::setup("mv_test_10_4", |dirs, sandbox| {
        sandbox.mkdir("mydir").mkdir("mydir/mydir_2");

        let actual = nu!(
            cwd: dirs.test(),
            "mv mydir mydir/mydir_2/"
        );

        assert!(actual.err.contains("cannot move to itself"));
    })
}

#[test]
fn does_not_error_on_relative_parent_path() {
    Playground::setup("mv_test_11", |dirs, sandbox| {
        sandbox
            .mkdir("first")
            .with_files(vec![EmptyFile("first/william_hartnell.txt")]);

        let original = dirs.test().join("first/william_hartnell.txt");
        let expected = dirs.test().join("william_hartnell.txt");

        nu!(
            cwd: dirs.test().join("first"),
            "mv william_hartnell.txt ./.."
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn move_files_using_glob_two_parents_up_using_multiple_dots() {
    Playground::setup("mv_test_12", |dirs, sandbox| {
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
                mv * ...
            "#
        );

        let files = vec![
            "yehuda.yaml",
            "jonathan.json",
            "andres.xml",
            "kevin.txt",
            "many_more.ppl",
        ];

        let original_dir = dirs.test().join("foo/bar");
        let destination_dir = dirs.test();

        assert!(files_exist_at(files.clone(), destination_dir));
        assert!(!files_exist_at(files, original_dir))
    })
}

#[test]
fn move_file_from_two_parents_up_using_multiple_dots_to_current_dir() {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("hello_there")]);
        sandbox.within("foo").mkdir("bar");

        nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                mv .../hello_there .
            "#
        );

        let expected = dirs.test().join("foo/bar/hello_there");
        let original = dirs.test().join("hello_there");

        assert!(expected.exists());
        assert!(!original.exists());
    })
}

#[test]
fn does_not_error_when_some_file_is_moving_into_itself() {
    Playground::setup("mv_test_13", |dirs, sandbox| {
        sandbox.mkdir("11").mkdir("12");

        let original_dir = dirs.test().join("11");
        let expected = dirs.test().join("12/11");
        nu!(cwd: dirs.test(), "mv 1* 12");

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn mv_ignores_ansi() {
    Playground::setup("mv_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("test.txt")]);
        let actual = nu!(
             cwd: sandbox.cwd(),
            r#"
                 ls | find test | mv $in.0.name success.txt; ls | $in.0.name
            "#
        );

        assert_eq!(actual.out, "success.txt");
    })
}

#[test]
fn mv_directory_with_same_name() {
    Playground::setup("mv_test_directory_with_same_name", |_dirs, sandbox| {
        sandbox.mkdir("testdir");
        sandbox.mkdir("testdir/testdir");

        let cwd = sandbox.cwd().join("testdir");
        let actual = nu!(
            cwd: cwd,
            r#"
                 mv testdir ..
            "#
        );

        assert!(actual.err.contains("Directory not empty"));
    })
}

#[test]
// Test that changing the case of a file/directory name works;
// this is an important edge case on Windows (and any other case-insensitive file systems).
// We were bitten badly by this once: https://github.com/nushell/nushell/issues/6583
fn mv_change_case_of_directory() {
    Playground::setup("mv_change_case_of_directory", |dirs, sandbox| {
        sandbox
            .mkdir("somedir")
            .with_files(vec![EmptyFile("somedir/somefile.txt")]);

        let original_dir = String::from("somedir");
        let new_dir = String::from("SomeDir");

        nu!(
            cwd: dirs.test(),
            format!("mv {original_dir} {new_dir}")
        );

        // Doing this instead of `Path::exists()` because we need to check file existence in
        // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
        let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())
            .unwrap()
            .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(!files_in_test_directory.contains(&original_dir));
        assert!(files_in_test_directory.contains(&new_dir));

        assert!(files_exist_at(
            vec!["somefile.txt",],
            dirs.test().join(new_dir)
        ));
    })
}

#[test]
fn mv_change_case_of_file() {
    Playground::setup("mv_change_case_of_file", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("somefile.txt")]);

        let original_file_name = String::from("somefile.txt");
        let new_file_name = String::from("SomeFile.txt");

        nu!(
            cwd: dirs.test(),
            format!("mv {original_file_name} -f {new_file_name}")
        );

        // Doing this instead of `Path::exists()` because we need to check file existence in
        // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
        let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())
            .unwrap()
            .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(!files_in_test_directory.contains(&original_file_name));
        assert!(files_in_test_directory.contains(&new_file_name));
    })
}
