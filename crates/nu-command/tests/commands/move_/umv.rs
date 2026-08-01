use nu_test_support::{
    fs::{Stub::EmptyFile, Stub::FileWithContent, files_exist_at},
    prelude::*,
};
use rstest::rstest;

const RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[test]
#[deps(NU)]
fn moves_a_file() -> Result {
    Playground::setup("umv_test_1", |dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile("andres.txt")])
            .mkdir("expected");

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("expected/yehuda.txt");

        let code = "mv andres.txt expected/yehuda.txt";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn overwrites_if_moving_to_existing_file_and_force_provided() -> Result {
    Playground::setup("umv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jttxt");

        let code = "mv andres.txt -f jttxt";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_a_directory() -> Result {
    Playground::setup("umv_test_3", |dirs, sandbox| {
        sandbox.mkdir("empty_dir");

        let original_dir = dirs.test().join("empty_dir");
        let expected = dirs.test().join("renamed_dir");

        let code = "mv empty_dir renamed_dir";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() -> Result {
    Playground::setup("umv_test_4", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("jttxt")]).mkdir("expected");

        let original_dir = dirs.test().join("jttxt");
        let expected = dirs.test().join("expected/jttxt");

        let code = "mv jttxt expected";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() -> Result {
    Playground::setup("umv_test_5", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(&[EmptyFile("jttxt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        let code = "mv contributors expected";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original_dir.exists());
        assert!(expected.exists());
        assert!(files_exist_at(&["jttxt"], expected));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_using_path_with_wildcard() -> Result {
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

        let code = "mv ../originals/*.ini ../expected";
        let result: CompleteResult = test().cwd(work_dir).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(files_exist_at(
            &["yehuda.ini", "jt.ini", "sample.ini", "andres.ini",],
            expected
        ));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_using_a_glob() -> Result {
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

        let code = "mv ../meals/* ../expected";
        let result: CompleteResult = test().cwd(work_dir).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(meal_dir.exists());
        assert!(files_exist_at(
            &["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn moves_a_directory_with_files() -> Result {
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

        let code = "mv vehicles expected";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

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
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn errors_if_source_doesnt_exist() -> Result {
    Playground::setup("umv_test_10", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        let code = "mv non-existing-file test_folder/";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_ne!(result.exit_code, 0);
        assert_contains("not_found", result.stderr);
        Ok(())
    })
}

#[test]
#[ignore = "GNU/uutils overwrites rather than error out"]
#[deps(NU)]
fn error_if_moving_to_existing_file_without_force() -> Result {
    Playground::setup("umv_test_10_0", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let code = "mv andres.txt jttxt";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_contains("file already exists", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn errors_if_destination_doesnt_exist() -> Result {
    Playground::setup("umv_test_10_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("empty.txt")]);

        let code = "mv empty.txt does/not/exist/";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        let msg = result.stderr;

        assert_contains("failed to access", &msg);
        assert_contains("Not a directory", msg);
        Ok(())
    })
}

#[test]
#[ignore = "GNU/uutils doesnt expand, rather cannot stat 'file?.txt'"]
#[deps(NU)]
fn errors_if_multiple_sources_but_destination_not_a_directory() -> Result {
    Playground::setup("umv_test_10_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("file1.txt"),
            EmptyFile("file2.txt"),
            EmptyFile("file3.txt"),
        ]);

        let code = "mv file?.txt not_a_dir";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;

        assert_contains(
            "Can only move multiple sources if destination is a directory",
            result.stderr,
        );
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn errors_if_renaming_directory_to_an_existing_file() -> Result {
    Playground::setup("umv_test_10_3", |dirs, sandbox| {
        sandbox.mkdir("mydir").with_files(&[EmptyFile("empty.txt")]);

        let code = "mv mydir empty.txt";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        let msg = result.stderr;
        assert_contains("cannot overwrite non-directory", &msg);
        assert_contains("with directory", msg);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn errors_if_moving_to_itself() -> Result {
    Playground::setup("umv_test_10_4", |dirs, sandbox| {
        sandbox.mkdir("mydir").mkdir("mydir/mydir_2");

        let code = "mv mydir mydir/mydir_2/";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        let msg = result.stderr;

        assert_contains("cannot move", &msg);
        assert_contains("to a subdirectory", msg);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn does_not_error_on_relative_parent_path() -> Result {
    Playground::setup("umv_test_11", |dirs, sandbox| {
        sandbox
            .mkdir("first")
            .with_files(&[EmptyFile("first/william_hartnell.txt")]);

        let original = dirs.test().join("first/william_hartnell.txt");
        let expected = dirs.test().join("william_hartnell.txt");

        let code = "mv william_hartnell.txt ./..";
        let result: CompleteResult = test()
            .cwd(dirs.test().join("first"))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn move_files_using_glob_two_parents_up_using_multiple_dots() -> Result {
    Playground::setup("umv_test_12", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(&[
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        let code = "mv * ...";
        let result: CompleteResult = test()
            .cwd(dirs.test().join("foo/bar"))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

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
        assert!(!files_exist_at(files, original_dir));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn move_file_from_two_parents_up_using_multiple_dots_to_current_dir() -> Result {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("hello_there")]);
        sandbox.within("foo").mkdir("bar");

        let code = "mv .../hello_there .";
        let result: CompleteResult = test()
            .cwd(dirs.test().join("foo/bar"))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let expected = dirs.test().join("foo/bar/hello_there");
        let original = dirs.test().join("hello_there");

        assert!(expected.exists());
        assert!(!original.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn does_not_error_when_some_file_is_moving_into_itself() -> Result {
    Playground::setup("umv_test_13", |dirs, sandbox| {
        sandbox.mkdir("11").mkdir("12");

        let original_dir = dirs.test().join("11");
        let expected = dirs.test().join("12/11");
        let code = "mv 1* 12";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_ignores_ansi() -> Result {
    Playground::setup("umv_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        let code = "ls | find test | mv $in.0.name success.txt; ls | $in.0.name";
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert_eq!(result.stdout.trim(), "success.txt");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_directory_with_same_name() -> Result {
    Playground::setup("umv_test_directory_with_same_name", |_dirs, sandbox| {
        sandbox.mkdir("testdir");
        sandbox.mkdir("testdir/testdir");

        let cwd = sandbox.cwd().join("testdir");
        let code = "mv testdir ..";
        let result: CompleteResult = test().cwd(&cwd).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(cwd.join("testdir").exists());
        Ok(())
    })
}

#[test]
// Test that changing the case of a file/directory name works;
// this is an important edge case on Windows (and any other case-insensitive file systems).
// We were bitten badly by this once: https://github.com/nushell/nushell/issues/6583

// Currently as we are using `uutils` and have no say in the behavior, this should succeed on Linux,
// but fail on both macOS and Windows.
#[deps(NU)]
fn mv_change_case_of_directory() -> Result {
    Playground::setup("mv_change_case_of_directory", |dirs, sandbox| {
        sandbox
            .mkdir("somedir")
            .with_files(&[EmptyFile("somedir/somefile.txt")]);

        let original_dir = String::from("somedir");
        let new_dir = String::from("SomeDir");

        let code = format!("mv {original_dir} {new_dir}");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

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
        {
            let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())?
                .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
                .collect();

            assert!(files_in_test_directory.contains(&original_dir));
        }
        Ok(())
    })
}

#[test]
// Currently as we are using `uutils` and have no say in the behavior, this is platform-dependent.
#[deps(NU)]
fn mv_change_case_of_file() -> Result {
    Playground::setup("mv_change_case_of_file", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("somefile.txt")]);

        let original_file_name = String::from("somefile.txt");
        let new_file_name = String::from("SomeFile.txt");

        let code = format!("mv {original_file_name} -f {new_file_name}");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

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
        {
            let files_in_test_directory: Vec<String> = std::fs::read_dir(dirs.test())?
                .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
                .collect();

            assert!(files_in_test_directory.contains(&new_file_name));
        }
        Ok(())
    })
}

#[test]
#[ignore = "Update not supported..remove later"]
#[deps(NU)]
fn mv_with_update_flag() -> Result {
    Playground::setup("umv_with_update_flag", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        let code = "mv -uf valid.txt newer_valid.txt; open newer_valid.txt";
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.stdout.trim(), "body");

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let code = "mv -uf newest_valid.txt valid.txt; open valid.txt";
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.stdout.trim(), "newest_body");

        // when destination doesn't exist
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let code = "mv -uf newest_valid.txt des_missing.txt; open des_missing.txt";
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.stdout.trim(), "newest_body");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_mv_no_clobber() -> Result {
    Playground::setup("umv_test_13", |dirs, sandbox| {
        let file_a = "test_mv_no_clobber_file_a";
        let file_b = "test_mv_no_clobber_file_b";
        sandbox.with_files(&[EmptyFile(file_a)]);
        sandbox.with_files(&[EmptyFile(file_b)]);

        let code = format!("mv -n {file_a} {file_b}");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let code = "ls test_mv* | length";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.stdout.trim(), "2");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_with_no_arguments() -> Result {
    Playground::setup("umv_test_14", |dirs, _| {
        let code = "mv";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_contains("Missing file operand", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_with_no_target() -> Result {
    Playground::setup("umv_test_15", |dirs, _| {
        let code = "mv a";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_contains("Missing destination path", result.stderr);
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg_attr(windows, ignore)]
#[case("a]?c")]
#[cfg_attr(windows, ignore)]
#[case("a*.?c")]
#[nu_test_support::test]
#[deps(NU)]
fn mv_files_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("umv_test_16", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let code = format!("mv '{}' {}", src.display(), "hello_world_dest");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(dirs.test().join("hello_world_dest").exists());
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg_attr(windows, ignore)]
#[case("a]?c")]
#[cfg_attr(windows, ignore)]
#[case("a*.?c")]
#[nu_test_support::test]
#[deps(NU)]
fn mv_files_with_glob_metachars_when_input_are_variables(#[case] src_name: &str) -> Result {
    Playground::setup("umv_test_18", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let code = format!("let f = '{}'; mv $f {}", src.display(), "hello_world_dest");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(dirs.test().join("hello_world_dest").exists());
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_with_cd() -> Result {
    Playground::setup("umv_test_17", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(&[FileWithContent("tmp_dir/file.txt", "body")]);

        let code = "do { cd tmp_dir; let f = 'file.txt'; mv $f .. }; open file.txt";
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.stdout.trim(), "body");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_mv_inside_glob_metachars_dir() -> Result {
    Playground::setup("uv_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        let code = "mv test_file.txt ../";
        let result: CompleteResult = test()
            .cwd(dirs.test().join(sub_dir))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        assert!(files_exist_at(&["test_file.txt"], dirs.test()));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_mv_wildcards() -> Result {
    Playground::setup("uv_with_wildcards", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent(".a", "hello")]);

        let code = "mv * ../";
        let result: CompleteResult = test()
            .cwd(dirs.test().join(sub_dir))
            .run_with_data(RUNNER, code)?;
        // by default, wildcard don't match dot files.
        assert_contains("File not found", result.stderr);
        assert!(files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(!files_exist_at(&[".a"], dirs.test()));

        // unless `-a` flag is provided.
        let code = "mv -a * ../";
        let result: CompleteResult = test()
            .cwd(dirs.test().join(sub_dir))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        // by default, wildcard don't match dot files.
        assert!(!files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(files_exist_at(&[".a"], dirs.test()));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_with_tilde() -> Result {
    Playground::setup("mv_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);
        sandbox.within("~tilde2");

        // mv file
        let code = "mv '~tilde/f1.txt' ./";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert!(!files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));

        // pass variable
        let code = "let f = '~tilde/f2.txt'; mv $f ./";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert!(!files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn mv_verbose_message_mentions_source_and_destination() -> Result {
    Playground::setup("umv_verbose_message", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("before.txt")]);

        let code = "mv -v before.txt after.txt";
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert_contains("before.txt", &result.stdout);
        assert_contains("after.txt", &result.stdout);
        assert!(dirs.test().join("after.txt").exists());
        assert!(!dirs.test().join("before.txt").exists());
        Ok(())
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
#[deps(NU)]
fn mv_literal_directory() -> Result {
    Playground::setup("mv_literal_dir_dc", |dirs, sandbox| {
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("test.txt")]);
        sandbox.mkdir("dest");

        let code = "mv mv_literal_dir_dc/subdir mv_literal_dir_dc/dest";
        let result: CompleteResult = test().cwd(dirs.root()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(!dirs.test().join("subdir").exists());
        assert!(dirs.test().join("dest/subdir/test.txt").exists());
        Ok(())
    })
}
