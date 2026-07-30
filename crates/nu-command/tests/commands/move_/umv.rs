use nu_test_support::{
    fs::{Stub::EmptyFile, Stub::FileWithContent, files_exist_at},
    prelude::*,
};
use rstest::rstest;

#[test]
fn moves_a_file() -> Result {
    Playground::setup("umv_test_1", |dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile("andres.txt")])
            .mkdir("expected");

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("expected/yehuda.txt");

        let () = test()
            .cwd(dirs.test())
            .run("mv andres.txt expected/yehuda.txt")?;

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn overwrites_if_moving_to_existing_file_and_force_provided() -> Result {
    Playground::setup("umv_test_2", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let original = dirs.test().join("andres.txt");
        let expected = dirs.test().join("jttxt");

        let () = test().cwd(dirs.test()).run("mv andres.txt -f jttxt")?;

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn moves_a_directory() -> Result {
    Playground::setup("umv_test_3", |dirs, sandbox| {
        sandbox.mkdir("empty_dir");

        let original_dir = dirs.test().join("empty_dir");
        let expected = dirs.test().join("renamed_dir");

        let () = test().cwd(dirs.test()).run("mv empty_dir renamed_dir")?;

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory() -> Result {
    Playground::setup("umv_test_4", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("jttxt")]).mkdir("expected");

        let original_dir = dirs.test().join("jttxt");
        let expected = dirs.test().join("expected/jttxt");

        let () = test().cwd(dirs.test()).run("mv jttxt expected")?;

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory() -> Result {
    Playground::setup("umv_test_5", |dirs, sandbox| {
        sandbox
            .within("contributors")
            .with_files(&[EmptyFile("jttxt")])
            .mkdir("expected");

        let original_dir = dirs.test().join("contributors");
        let expected = dirs.test().join("expected/contributors");

        let () = test().cwd(dirs.test()).run("mv contributors expected")?;

        assert!(!original_dir.exists());
        assert!(expected.exists());
        assert!(files_exist_at(&["jttxt"], expected));
        Ok(())
    })
}

#[test]
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

        let () = test()
            .cwd(work_dir)
            .run("mv ../originals/*.ini ../expected")?;

        assert!(files_exist_at(
            &["yehuda.ini", "jt.ini", "sample.ini", "andres.ini",],
            expected
        ));
        Ok(())
    })
}

#[test]
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

        let () = test().cwd(work_dir).run("mv ../meals/* ../expected")?;

        assert!(meal_dir.exists());
        assert!(files_exist_at(
            &["arepa.txt", "empanada.txt", "taquiza.txt",],
            expected
        ));
        Ok(())
    })
}

#[test]
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

        let () = test().cwd(dirs.test()).run("mv vehicles expected")?;

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
fn errors_if_source_doesnt_exist() -> Result {
    Playground::setup("umv_test_10", |dirs, sandbox| {
        sandbox.mkdir("test_folder");
        test()
            .cwd(dirs.test())
            .run("mv non-existing-file test_folder/")
            .expect_error_code_eq("nu::shell::io::not_found")
    })
}

#[test]
#[ignore = "GNU/uutils overwrites rather than error out"]
fn error_if_moving_to_existing_file_without_force() -> Result {
    Playground::setup("umv_test_10_0", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("andres.txt"), EmptyFile("jttxt")]);

        let err = test()
            .cwd(dirs.test())
            .run("mv andres.txt jttxt")
            .expect_shell_error()?;
        assert_contains("file already exists", err.to_string());
        Ok(())
    })
}

#[test]
fn errors_if_destination_doesnt_exist() -> Result {
    Playground::setup("umv_test_10_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("empty.txt")]);

        let err = test()
            .cwd(dirs.test())
            .run("mv empty.txt does/not/exist/")
            .expect_shell_error()?;
        let msg = err.to_string();

        assert_contains("failed to access", &msg);
        assert_contains("Not a directory", msg);
        Ok(())
    })
}

#[test]
#[ignore = "GNU/uutils doesnt expand, rather cannot stat 'file?.txt'"]
fn errors_if_multiple_sources_but_destination_not_a_directory() -> Result {
    Playground::setup("umv_test_10_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("file1.txt"),
            EmptyFile("file2.txt"),
            EmptyFile("file3.txt"),
        ]);

        let err = test()
            .cwd(dirs.test())
            .run("mv file?.txt not_a_dir")
            .expect_shell_error()?;

        assert_contains(
            "Can only move multiple sources if destination is a directory",
            err.to_string(),
        );
        Ok(())
    })
}

#[test]
fn errors_if_renaming_directory_to_an_existing_file() -> Result {
    Playground::setup("umv_test_10_3", |dirs, sandbox| {
        sandbox.mkdir("mydir").with_files(&[EmptyFile("empty.txt")]);

        let err = test()
            .cwd(dirs.test())
            .run("mv mydir empty.txt")
            .expect_shell_error()?;
        let msg = err.to_string();
        assert_contains("cannot overwrite non-directory", &msg);
        assert_contains("with directory", msg);
        Ok(())
    })
}

#[test]
fn errors_if_moving_to_itself() -> Result {
    Playground::setup("umv_test_10_4", |dirs, sandbox| {
        sandbox.mkdir("mydir").mkdir("mydir/mydir_2");

        let err = test()
            .cwd(dirs.test())
            .run("mv mydir mydir/mydir_2/")
            .expect_shell_error()?;
        let msg = err.to_string();

        assert_contains("cannot move", &msg);
        assert_contains("to a subdirectory", msg);
        Ok(())
    })
}

#[test]
fn does_not_error_on_relative_parent_path() -> Result {
    Playground::setup("umv_test_11", |dirs, sandbox| {
        sandbox
            .mkdir("first")
            .with_files(&[EmptyFile("first/william_hartnell.txt")]);

        let original = dirs.test().join("first/william_hartnell.txt");
        let expected = dirs.test().join("william_hartnell.txt");

        let () = test()
            .cwd(dirs.test().join("first"))
            .run("mv william_hartnell.txt ./..")?;

        assert!(!original.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn move_files_using_glob_two_parents_up_using_multiple_dots() -> Result {
    Playground::setup("umv_test_12", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(&[
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        let () = test().cwd(dirs.test().join("foo/bar")).run("mv * ...")?;

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
fn move_file_from_two_parents_up_using_multiple_dots_to_current_dir() -> Result {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("hello_there")]);
        sandbox.within("foo").mkdir("bar");

        let () = test()
            .cwd(dirs.test().join("foo/bar"))
            .run("mv .../hello_there .")?;

        let expected = dirs.test().join("foo/bar/hello_there");
        let original = dirs.test().join("hello_there");

        assert!(expected.exists());
        assert!(!original.exists());
        Ok(())
    })
}

#[test]
fn does_not_error_when_some_file_is_moving_into_itself() -> Result {
    Playground::setup("umv_test_13", |dirs, sandbox| {
        sandbox.mkdir("11").mkdir("12");

        let original_dir = dirs.test().join("11");
        let expected = dirs.test().join("12/11");
        let () = test().cwd(dirs.test()).run("mv 1* 12")?;

        assert!(!original_dir.exists());
        assert!(expected.exists());
        Ok(())
    })
}

#[test]
fn mv_ignores_ansi() -> Result {
    Playground::setup("umv_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        test()
            .cwd(sandbox.cwd())
            .run("ls | find test | mv $in.0.name success.txt; ls | $in.0.name")
            .expect_value_eq("success.txt")
    })
}

#[test]
fn mv_directory_with_same_name() -> Result {
    Playground::setup("umv_test_directory_with_same_name", |_dirs, sandbox| {
        sandbox.mkdir("testdir");
        sandbox.mkdir("testdir/testdir");

        let cwd = sandbox.cwd().join("testdir");
        let () = test().cwd(&cwd).run("mv testdir ..")?;

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
fn mv_change_case_of_directory() -> Result {
    Playground::setup("mv_change_case_of_directory", |dirs, sandbox| {
        sandbox
            .mkdir("somedir")
            .with_files(&[EmptyFile("somedir/somefile.txt")]);

        let original_dir = String::from("somedir");
        let new_dir = String::from("SomeDir");

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let () = test()
            .cwd(dirs.test())
            .run(format!("mv {original_dir} {new_dir}"))?;

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        let () = test()
            .cwd(dirs.test())
            .run(format!("mv {original_dir} {new_dir}"))?;

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
fn mv_change_case_of_file() -> Result {
    Playground::setup("mv_change_case_of_file", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("somefile.txt")]);

        let original_file_name = String::from("somefile.txt");
        let new_file_name = String::from("SomeFile.txt");

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        let () = test()
            .cwd(dirs.test())
            .run(format!("mv {original_file_name} -f {new_file_name}"))?;

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        let () = test()
            .cwd(dirs.test())
            .run(format!("mv {original_file_name} -f {new_file_name}"))?;

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
fn mv_with_update_flag() -> Result {
    Playground::setup("umv_with_update_flag", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        test()
            .cwd(sandbox.cwd())
            .run("mv -uf valid.txt newer_valid.txt; open newer_valid.txt")
            .expect_value_eq("body")?;

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        test()
            .cwd(sandbox.cwd())
            .run("mv -uf newest_valid.txt valid.txt; open valid.txt")
            .expect_value_eq("newest_body")?;

        // when destination doesn't exist
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        test()
            .cwd(sandbox.cwd())
            .run("mv -uf newest_valid.txt des_missing.txt; open des_missing.txt")
            .expect_value_eq("newest_body")?;
        Ok(())
    })
}

#[test]
fn test_mv_no_clobber() -> Result {
    Playground::setup("umv_test_13", |dirs, sandbox| {
        let file_a = "test_mv_no_clobber_file_a";
        let file_b = "test_mv_no_clobber_file_b";
        sandbox.with_files(&[EmptyFile(file_a)]);
        sandbox.with_files(&[EmptyFile(file_b)]);

        let () = test()
            .cwd(dirs.test())
            .run(format!("mv -n {file_a} {file_b}"))?;

        test()
            .cwd(dirs.test())
            .run("ls test_mv* | length")
            .expect_value_eq(2)
    })
}

#[test]
fn mv_with_no_arguments() -> Result {
    Playground::setup("umv_test_14", |dirs, _| {
        let err = test().cwd(dirs.test()).run("mv").expect_shell_error()?;
        assert_contains("Missing file operand", err.to_string());
        Ok(())
    })
}

#[test]
fn mv_with_no_target() -> Result {
    Playground::setup("umv_test_15", |dirs, _| {
        let err = test().cwd(dirs.test()).run("mv a").expect_shell_error()?;
        assert_contains("Missing destination path", err.to_string());
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn mv_files_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("umv_test_16", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let () = test().cwd(dirs.test()).run(format!(
            "mv '{}' {}",
            src.display(),
            "hello_world_dest"
        ))?;

        assert!(dirs.test().join("hello_world_dest").exists());
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn mv_files_with_glob_metachars_when_input_are_variables(#[case] src_name: &str) -> Result {
    Playground::setup("umv_test_18", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let () = test().cwd(dirs.test()).run(format!(
            "let f = '{}'; mv $f {}",
            src.display(),
            "hello_world_dest"
        ))?;

        assert!(dirs.test().join("hello_world_dest").exists());
        Ok(())
    })
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn mv_files_with_glob_metachars_nw(#[case] src_name: &str) -> Result {
    mv_files_with_glob_metachars(src_name)?;
    mv_files_with_glob_metachars_when_input_are_variables(src_name)
}

#[test]
fn mv_with_cd() -> Result {
    Playground::setup("umv_test_17", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(&[FileWithContent("tmp_dir/file.txt", "body")]);

        test()
            .cwd(sandbox.cwd())
            .run("do { cd tmp_dir; let f = 'file.txt'; mv $f .. }; open file.txt")
            .expect_value_eq("body")
    })
}

#[test]
fn test_mv_inside_glob_metachars_dir() -> Result {
    Playground::setup("uv_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        let () = test()
            .cwd(dirs.test().join(sub_dir))
            .run("mv test_file.txt ../")?;

        assert!(!files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        assert!(files_exist_at(&["test_file.txt"], dirs.test()));
        Ok(())
    })
}

#[test]
fn test_mv_wildcards() -> Result {
    Playground::setup("uv_with_wildcards", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent(".a", "hello")]);

        let err = test()
            .cwd(dirs.test().join(sub_dir))
            .run("mv * ../")
            .expect_shell_error()?;
        // by default, wildcard don't match dot files.
        assert_contains("File not found", err.to_string());
        assert!(files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(!files_exist_at(&[".a"], dirs.test()));

        // unless `-a` flag is provided.
        let () = test().cwd(dirs.test().join(sub_dir)).run("mv -a * ../")?;
        // by default, wildcard don't match dot files.
        assert!(!files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(files_exist_at(&[".a"], dirs.test()));
        Ok(())
    })
}

#[test]
fn mv_with_tilde() -> Result {
    Playground::setup("mv_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);
        sandbox.within("~tilde2");

        // mv file
        let () = test().cwd(dirs.test()).run("mv '~tilde/f1.txt' ./")?;
        assert!(!files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));

        // pass variable
        let () = test()
            .cwd(dirs.test())
            .run("let f = '~tilde/f2.txt'; mv $f ./")?;
        assert!(!files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));
        Ok(())
    })
}

#[test]
fn mv_verbose_message_mentions_source_and_destination() -> Result {
    Playground::setup("umv_verbose_message", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("before.txt")]);

        let message: String = test()
            .cwd(dirs.test())
            .run("mv -v before.txt after.txt | get 0.message")?;

        assert_contains("before.txt", &message);
        assert_contains("after.txt", message);
        assert!(dirs.test().join("after.txt").exists());
        assert!(!dirs.test().join("before.txt").exists());
        Ok(())
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn mv_literal_directory() -> Result {
    Playground::setup("mv_literal_dir_dc", |dirs, sandbox| {
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("test.txt")]);
        sandbox.mkdir("dest");

        let () = test()
            .cwd(dirs.root())
            .run("mv mv_literal_dir_dc/subdir mv_literal_dir_dc/dest")?;

        assert!(!dirs.test().join("subdir").exists());
        assert!(dirs.test().join("dest/subdir/test.txt").exists());
        Ok(())
    })
}
