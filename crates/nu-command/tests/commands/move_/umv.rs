use nu_test_support::fs::{Stub::EmptyFile, Stub::FileWithContent, files_exist_at};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use rstest::rstest;

#[test]
fn moves_a_file() {
    Playground::setup("umv_test_1", |dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile("andres.txt")])
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
    Playground::setup("umv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jttxt");

        nu!(
            cwd: dirs.test(),
            "mv andres.txt -f jttxt"
        );

        assert!(!original.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_a_directory() {
    Playground::setup("umv_test_3", |dirs, sandbox| {
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
    Playground::setup("umv_test_4", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("jttxt")]).mkdir("expected");

        let original_dir = dirs.test().join("jttxt");
        let expected = dirs.test().join("expected/jttxt");

        nu!(
            cwd: dirs.test(),
            "mv jttxt expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() {
    Playground::setup("umv_test_5", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(&[EmptyFile("jttxt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        nu!(
            cwd: dirs.test(),
            "mv contributors expected"
        );

        assert!(!original_dir.exists());
        assert!(expected.exists());
        assert!(files_exist_at(&["jttxt"], expected))
    })
}

#[test]
fn moves_using_path_with_wildcard() {
    Playground::setup("umv_test_7", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(&[
                EmptyFile("andres.ini"),
                EmptyFile("caco3_plastics.csv"),
                EmptyFile("cargo_sample.toml"),
                EmptyFile("jt.ini"),
                EmptyFile("jt.xml"),
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
            &["yehuda.ini", "jt.ini", "sample.ini", "andres.ini",],
            expected
        ));
    })
}

#[test]
fn moves_using_a_glob() {
    Playground::setup("umv_test_8", |dirs, sandbox| {
        sandbox
            .within("meals")
            .with_files(&[
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
            &["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
    })
}

#[test]
fn moves_a_directory_with_files() {
    Playground::setup("umv_test_9", |dirs, sandbox| {
        sandbox
            .mkdir("vehicles/car")
            .mkdir("vehicles/bicycle")
            .with_files(&[
                EmptyFile("vehicles/car/car1.txt"),
                EmptyFile("vehicles/car/car2.txt"),
            ])
            .with_files(&[
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
            &[
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
    Playground::setup("umv_test_10", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let actual = nu!(
            cwd: dirs.test(),
            "mv non-existing-file test_folder/"
        );
        assert!(actual.err.contains("nu::shell::io::not_found"));
    })
}

#[test]
#[ignore = "GNU/uutils overwrites rather than error out"]
fn error_if_moving_to_existing_file_without_force() {
    Playground::setup("umv_test_10_0", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv andres.txt jttxt"
        );
        assert!(actual.err.contains("file already exists"))
    })
}

#[test]
fn errors_if_destination_doesnt_exist() {
    Playground::setup("umv_test_10_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("empty.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv empty.txt does/not/exist/"
        );

        assert!(actual.err.contains("failed to access"));
        assert!(actual.err.contains("Not a directory"));
    })
}

#[test]
#[ignore = "GNU/uutils doesnt expand, rather cannot stat 'file?.txt'"]
fn errors_if_multiple_sources_but_destination_not_a_directory() {
    Playground::setup("umv_test_10_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("file1.txt"),
            EmptyFile("file2.txt"),
            EmptyFile("file3.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv file?.txt not_a_dir"
        );

        assert!(
            actual
                .err
                .contains("Can only move multiple sources if destination is a directory")
        );
    })
}

#[test]
fn errors_if_renaming_directory_to_an_existing_file() {
    Playground::setup("umv_test_10_3", |dirs, sandbox| {
        sandbox.mkdir("mydir").with_files(&[EmptyFile("empty.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "mv mydir empty.txt"
        );
        assert!(actual.err.contains("cannot overwrite non-directory"),);
        assert!(actual.err.contains("with directory"),);
    })
}

#[test]
fn errors_if_moving_to_itself() {
    Playground::setup("umv_test_10_4", |dirs, sandbox| {
        sandbox.mkdir("mydir").mkdir("mydir/mydir_2");

        let actual = nu!(
            cwd: dirs.test(),
            "mv mydir mydir/mydir_2/"
        );

        assert!(actual.err.contains("cannot move"));
        assert!(actual.err.contains("to a subdirectory"));
    });
}

#[test]
fn does_not_error_on_relative_parent_path() {
    Playground::setup("umv_test_11", |dirs, sandbox| {
        sandbox
            .mkdir("first")
            .with_files(&[EmptyFile("first/william_hartnell.txt")]);

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
    Playground::setup("umv_test_12", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(&[
            EmptyFile("jtjson"),
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

        let files = &[
            "yehuda.yaml",
            "jtjson",
            "andres.xml",
            "kevin.txt",
            "many_more.ppl",
        ];

        let original_dir = dirs.test().join("foo/bar");
        let destination_dir = dirs.test();

        assert!(files_exist_at(files, destination_dir));
        assert!(!files_exist_at(files, original_dir))
    })
}

#[test]
fn move_file_from_two_parents_up_using_multiple_dots_to_current_dir() {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("hello_there")]);
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
    Playground::setup("umv_test_13", |dirs, sandbox| {
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
    Playground::setup("umv_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);
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
    Playground::setup("umv_test_directory_with_same_name", |_dirs, sandbox| {
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

// Currently as we are using `uutils` and have no say in the behavior, this should succeed on Linux,
// but fail on both macOS and Windows.
fn mv_change_case_of_directory() {
    Playground::setup("mv_change_case_of_directory", |dirs, sandbox| {
        sandbox
            .mkdir("somedir")
            .with_files(&[EmptyFile("somedir/somefile.txt")]);

        let original_dir = String::from("somedir");
        let new_dir = String::from("SomeDir");

        #[allow(unused)]
        let actual = nu!(
            cwd: dirs.test(),
            format!("mv {original_dir} {new_dir}")
        );

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            // Doing this instead of `Path::exists()` because we need to check file existence in
            // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
            let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())
                .unwrap()
                .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
                .collect();

            assert!(
                !files_in_test_directory.contains(&original_dir)
                    && files_in_test_directory.contains(&new_dir)
            );

            assert!(files_exist_at(&["somefile.txt"], dirs.test().join(new_dir)));
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        actual.err.contains("to a subdirectory of itself");
    })
}

#[test]
// Currently as we are using `uutils` and have no say in the behavior, this should succeed on Linux,
// but fail on both macOS and Windows.
fn mv_change_case_of_file() {
    Playground::setup("mv_change_case_of_file", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("somefile.txt")]);

        let original_file_name = String::from("somefile.txt");
        let new_file_name = String::from("SomeFile.txt");

        #[allow(unused)]
        let actual = nu!(
            cwd: dirs.test(),
            format!("mv {original_file_name} -f {new_file_name}")
        );

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            // Doing this instead of `Path::exists()` because we need to check file existence in
            // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
            let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())
                .unwrap()
                .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            assert!(
                !files_in_test_directory.contains(&original_file_name)
                    && files_in_test_directory.contains(&new_file_name)
            );
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        actual.err.contains("are the same file");
    })
}

#[test]
#[ignore = "Update not supported..remove later"]
fn mv_with_update_flag() {
    Playground::setup("umv_with_update_flag", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "mv -uf valid.txt newer_valid.txt; open newer_valid.txt",
        );
        assert_eq!(actual.out, "body");

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let actual = nu!(cwd: sandbox.cwd(), "mv -uf newest_valid.txt valid.txt; open valid.txt");
        assert_eq!(actual.out, "newest_body");

        // when destination doesn't exist
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let actual = nu!(cwd: sandbox.cwd(), "mv -uf newest_valid.txt des_missing.txt; open des_missing.txt");
        assert_eq!(actual.out, "newest_body");
    });
}

#[test]
fn test_mv_no_clobber() {
    Playground::setup("umv_test_13", |dirs, sandbox| {
        let file_a = "test_mv_no_clobber_file_a";
        let file_b = "test_mv_no_clobber_file_b";
        sandbox.with_files(&[EmptyFile(file_a)]);
        sandbox.with_files(&[EmptyFile(file_b)]);

        let _ = nu!(cwd: dirs.test(), format!("mv -n {file_a} {file_b}"));

        let file_count = nu!(
            cwd: dirs.test(),
            "ls test_mv* | length | to nuon"
        );
        assert_eq!(file_count.out, "2");
    })
}

#[test]
fn mv_with_no_arguments() {
    Playground::setup("umv_test_14", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "mv",
        );
        assert!(actual.err.contains("Missing file operand"));
    })
}

#[test]
fn mv_with_no_target() {
    Playground::setup("umv_test_15", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "mv a",
        );
        assert!(
            actual.err.contains(
                format!(
                    "Missing destination path operand after {}",
                    dirs.test().join("a").display()
                )
                .as_str()
            )
        );
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn mv_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("umv_test_16", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "mv '{}' {}",
                src.display(),
                "hello_world_dest"
            )
        );

        assert!(actual.err.is_empty());
        assert!(dirs.test().join("hello_world_dest").exists());
    });
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn mv_files_with_glob_metachars_when_input_are_variables(#[case] src_name: &str) {
    Playground::setup("umv_test_18", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "let f = '{}'; mv $f {}",
                src.display(),
                "hello_world_dest"
            )
        );

        assert!(actual.err.is_empty());
        assert!(dirs.test().join("hello_world_dest").exists());
    });
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn mv_files_with_glob_metachars_nw(#[case] src_name: &str) {
    mv_files_with_glob_metachars(src_name);
    mv_files_with_glob_metachars_when_input_are_variables(src_name);
}

#[test]
fn mv_with_cd() {
    Playground::setup("umv_test_17", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(&[FileWithContent("tmp_dir/file.txt", "body")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            r#"do { cd tmp_dir; let f = 'file.txt'; mv $f .. }; open file.txt"#,
        );
        assert!(actual.out.contains("body"));
    });
}

#[test]
fn test_cp_inside_glob_metachars_dir() {
    Playground::setup("open_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        let actual = nu!(
            cwd: dirs.test().join(sub_dir),
            "mv test_file.txt ../",
        );

        assert!(actual.err.is_empty());
        assert!(!files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        assert!(files_exist_at(&["test_file.txt"], dirs.test()));
    });
}

#[test]
fn mv_with_tilde() {
    Playground::setup("mv_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);
        sandbox.within("~tilde2");

        // mv file
        let actual = nu!(cwd: dirs.test(), "mv '~tilde/f1.txt' ./");
        assert!(actual.err.is_empty());
        assert!(!files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde/f2.txt'; mv $f ./");
        assert!(actual.err.is_empty());
        assert!(!files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));
    })
}
