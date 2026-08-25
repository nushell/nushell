use nu_test_support::prelude::*;
use rstest::rstest;

const RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[test]
#[deps(NU)]
fn moves_a_file(playground: Playground) -> Result {
    playground.empty_file("andres.txt")?;
    playground.dir("expected")?;

    let original = playground.path().join("andres.txt");
    let expected = playground.path().join("expected/yehuda.txt");

    let code = "mv andres.txt expected/yehuda.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn overwrites_if_moving_to_existing_file_and_force_provided(playground: Playground) -> Result {
    playground.empty_file("andres.txt")?;
    playground.empty_file("jttxt")?;

    let original = playground.path().join("andres.txt");
    let expected = playground.path().join("jttxt");

    let code = "mv andres.txt -f jttxt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_a_directory(playground: Playground) -> Result {
    playground.dir("empty_dir")?;

    let original_dir = playground.path().join("empty_dir");
    let expected = playground.path().join("renamed_dir");

    let code = "mv empty_dir renamed_dir";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original_dir.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_the_file_inside_directory_if_path_to_move_is_existing_directory(
    playground: Playground,
) -> Result {
    playground.empty_file("jttxt")?;
    playground.dir("expected")?;

    let original_dir = playground.path().join("jttxt");
    let expected = playground.path().join("expected/jttxt");

    let code = "mv jttxt expected";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original_dir.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_the_directory_inside_directory_if_path_to_move_is_existing_directory(
    playground: Playground,
) -> Result {
    playground.empty_file("contributors/jttxt")?;
    playground.dir("expected")?;

    let original_dir = playground.path().join("contributors");
    let expected = playground.path().join("expected/contributors");

    let code = "mv contributors expected";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original_dir.exists());
    assert!(expected.exists());
    assert!(expected.join("jttxt").exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_using_path_with_wildcard(playground: Playground) -> Result {
    for file in [
        "andres.ini",
        "caco3_plastics.csv",
        "cargo_sample.toml",
        "jt.ini",
        "jt.xml",
        "sgml_description.json",
        "sample.ini",
        "utf16.ini",
        "yehuda.ini",
    ] {
        playground.empty_file(format!("originals/{file}"))?;
    }
    playground.dir("work_dir")?;
    playground.dir("expected")?;

    let work_dir = playground.path().join("work_dir");
    let expected = playground.path().join("expected");

    let code = "mv ../originals/*.ini ../expected";
    let result: CompleteResult = test().cwd(work_dir).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(
        ["yehuda.ini", "jt.ini", "sample.ini", "andres.ini"]
            .iter()
            .all(|file| expected.join(file).exists())
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_using_a_glob(playground: Playground) -> Result {
    for file in ["arepa.txt", "empanada.txt", "taquiza.txt"] {
        playground.empty_file(format!("meals/{file}"))?;
    }
    playground.dir("work_dir")?;
    playground.dir("expected")?;

    let meal_dir = playground.path().join("meals");
    let work_dir = playground.path().join("work_dir");
    let expected = playground.path().join("expected");

    let code = "mv ../meals/* ../expected";
    let result: CompleteResult = test().cwd(work_dir).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(meal_dir.exists());
    assert!(
        ["arepa.txt", "empanada.txt", "taquiza.txt"]
            .iter()
            .all(|file| expected.join(file).exists())
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn moves_a_directory_with_files(playground: Playground) -> Result {
    for file in [
        "vehicles/car/car1.txt",
        "vehicles/car/car2.txt",
        "vehicles/bicycle/bicycle1.txt",
        "vehicles/bicycle/bicycle2.txt",
    ] {
        playground.empty_file(file)?;
    }

    let original_dir = playground.path().join("vehicles");
    let expected_dir = playground.path().join("expected");

    let code = "mv vehicles expected";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original_dir.exists());
    assert!(expected_dir.exists());
    assert!(
        [
            "car/car1.txt",
            "car/car2.txt",
            "bicycle/bicycle1.txt",
            "bicycle/bicycle2.txt"
        ]
        .iter()
        .all(|file| expected_dir.join(file).exists())
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn errors_if_source_doesnt_exist(playground: Playground) -> Result {
    playground.dir("test_folder")?;
    let code = "mv non-existing-file test_folder/";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_ne!(result.exit_code, 0);
    assert_contains("not_found", result.stderr);
    Ok(())
}

#[test]
#[ignore = "GNU/uutils overwrites rather than error out"]
#[deps(NU)]
fn error_if_moving_to_existing_file_without_force(playground: Playground) -> Result {
    playground.empty_file("andres.txt")?;
    playground.empty_file("jttxt")?;

    let code = "mv andres.txt jttxt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_contains("file already exists", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn errors_if_destination_doesnt_exist(playground: Playground) -> Result {
    playground.empty_file("empty.txt")?;

    let code = "mv empty.txt does/not/exist/";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    let msg = result.stderr;

    assert_contains("failed to access", &msg);
    assert_contains("Not a directory", msg);
    Ok(())
}

#[test]
#[ignore = "GNU/uutils doesnt expand, rather cannot stat 'file?.txt'"]
#[deps(NU)]
fn errors_if_multiple_sources_but_destination_not_a_directory(playground: Playground) -> Result {
    playground.empty_file("file1.txt")?;
    playground.empty_file("file2.txt")?;
    playground.empty_file("file3.txt")?;

    let code = "mv file?.txt not_a_dir";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;

    assert_contains(
        "Can only move multiple sources if destination is a directory",
        result.stderr,
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn errors_if_renaming_directory_to_an_existing_file(playground: Playground) -> Result {
    playground.dir("mydir")?;
    playground.empty_file("empty.txt")?;

    let code = "mv mydir empty.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    let msg = result.stderr;
    assert_contains("cannot overwrite non-directory", &msg);
    assert_contains("with directory", msg);
    Ok(())
}

#[test]
#[deps(NU)]
fn errors_if_moving_to_itself(playground: Playground) -> Result {
    playground.dir("mydir/mydir_2")?;

    let code = "mv mydir mydir/mydir_2/";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    let msg = result.stderr;

    assert_contains("cannot move", &msg);
    assert_contains("to a subdirectory", msg);
    Ok(())
}

#[test]
#[deps(NU)]
fn does_not_error_on_relative_parent_path(playground: Playground) -> Result {
    playground.empty_file("first/william_hartnell.txt")?;

    let original = playground.path().join("first/william_hartnell.txt");
    let expected = playground.path().join("william_hartnell.txt");

    let code = "mv william_hartnell.txt ./..";
    let result: CompleteResult = test()
        .cwd(playground.path().join("first"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn move_files_using_glob_two_parents_up_using_multiple_dots(playground: Playground) -> Result {
    let files = [
        "yehuda.yaml",
        "jtjson",
        "andres.xml",
        "kevin.txt",
        "many_more.ppl",
    ];
    for file in files {
        playground.empty_file(format!("foo/bar/{file}"))?;
    }

    let code = "mv * ...";
    let result: CompleteResult = test()
        .cwd(playground.path().join("foo/bar"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let original_dir = playground.path().join("foo/bar");
    let destination_dir = playground.path();

    assert!(files.iter().all(|file| destination_dir.join(file).exists()));
    assert!(!files.iter().all(|file| original_dir.join(file).exists()));
    Ok(())
}

#[test]
#[deps(NU)]
fn move_file_from_two_parents_up_using_multiple_dots_to_current_dir(
    playground: Playground,
) -> Result {
    playground.empty_file("hello_there")?;
    playground.dir("foo/bar")?;

    let code = "mv .../hello_there .";
    let result: CompleteResult = test()
        .cwd(playground.path().join("foo/bar"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let expected = playground.path().join("foo/bar/hello_there");
    let original = playground.path().join("hello_there");

    assert!(expected.exists());
    assert!(!original.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn does_not_error_when_some_file_is_moving_into_itself(playground: Playground) -> Result {
    playground.dir("11")?;
    playground.dir("12")?;

    let original_dir = playground.path().join("11");
    let expected = playground.path().join("12/11");
    let code = "mv 1* 12";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!original_dir.exists());
    assert!(expected.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_ignores_ansi(playground: Playground) -> Result {
    playground.empty_file("test.txt")?;

    let code = "ls | find test | mv $in.0.name success.txt; ls | $in.0.name";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout.trim(), "success.txt");
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_directory_with_same_name(playground: Playground) -> Result {
    playground.dir("testdir")?;
    playground.dir("testdir/testdir")?;

    let cwd = playground.path().join("testdir");
    let code = "mv testdir ..";
    let result: CompleteResult = test().cwd(&cwd).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(cwd.join("testdir").exists());
    Ok(())
}

// Test that changing the case of a file/directory name works;
// this is an important edge case on Windows (and any other case-insensitive file systems).
// We were bitten badly by this once: https://github.com/nushell/nushell/issues/6583
//
// Currently as we are using `uutils` and have no say in the behavior, this should succeed on Linux,
// but fail on both macOS and Windows.
#[test]
#[deps(NU)]
#[cfg_attr(target_os = "macos", ignore)]
fn mv_change_case_of_directory(playground: Playground) -> Result {
    playground.empty_file("somedir/somefile.txt")?;

    let original_dir = String::from("somedir");
    let new_dir = String::from("SomeDir");

    let code = format!("mv {original_dir} {new_dir}");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // Doing this instead of `Path::exists()` because we need to check file existence in
        // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
        let files_in_test_directory: Vec<String> = std::fs::read_dir(playground.path())
            .unwrap()
            .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert!(
            !files_in_test_directory.contains(&original_dir)
                && files_in_test_directory.contains(&new_dir)
        );

        assert!(
            playground
                .path()
                .join(new_dir)
                .join("somefile.txt")
                .exists()
        );
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    {
        let files_in_test_directory: Vec<String> = std::fs::read_dir(playground.path())?
            .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert!(files_in_test_directory.contains(&original_dir));
    }
    Ok(())
}

// Currently as we are using `uutils` and have no say in the behavior, this is platform-dependent.
#[test]
#[deps(NU)]
#[cfg_attr(target_os = "macos", ignore)]
fn mv_change_case_of_file(playground: Playground) -> Result {
    playground.empty_file("somefile.txt")?;

    let original_file_name = String::from("somefile.txt");
    let new_file_name = String::from("SomeFile.txt");

    let code = format!("mv {original_file_name} -f {new_file_name}");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // Doing this instead of `Path::exists()` because we need to check file existence in
        // a case-sensitive way. `Path::exists()` is understandably case-insensitive on NTFS
        let files_in_test_directory: Vec<String> = std::fs::read_dir(playground.path())
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
        let files_in_test_directory: Vec<String> = std::fs::read_dir(playground.path())?
            .map(|de| de.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert!(files_in_test_directory.contains(&new_file_name));
    }
    Ok(())
}

#[test]
#[ignore = "Update not supported..remove later"]
#[deps(NU)]
fn mv_with_update_flag(playground: Playground) -> Result {
    playground.empty_file("valid.txt")?;
    playground.file("newer_valid.txt", "body")?;

    let code = "mv -uf valid.txt newer_valid.txt; open newer_valid.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.stdout.trim(), "body");

    // create a file after assert to make sure that newest_valid.txt is newest
    std::thread::sleep(std::time::Duration::from_secs(1));
    playground.file("newest_valid.txt", "newest_body")?;
    let code = "mv -uf newest_valid.txt valid.txt; open valid.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.stdout.trim(), "newest_body");

    // when destination doesn't exist
    playground.file("newest_valid.txt", "newest_body")?;
    let code = "mv -uf newest_valid.txt des_missing.txt; open des_missing.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.stdout.trim(), "newest_body");
    Ok(())
}

#[test]
#[deps(NU)]
fn test_mv_no_clobber(playground: Playground) -> Result {
    let file_a = "test_mv_no_clobber_file_a";
    let file_b = "test_mv_no_clobber_file_b";
    playground.empty_file(file_a)?;
    playground.empty_file(file_b)?;

    let code = format!("mv -n {file_a} {file_b}");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let code = "ls test_mv* | length";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.stdout.trim(), "2");
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_with_no_arguments(playground: Playground) -> Result {
    let code = "mv";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_contains("Missing file operand", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_with_no_target(playground: Playground) -> Result {
    let code = "mv a";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_contains("Missing destination path", result.stderr);
    Ok(())
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
fn mv_files_with_glob_metachars(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    playground.file(src_name, "What is the sound of one hand clapping?")?;

    let src = playground.path().join(src_name);

    let code = format!("mv '{}' {}", src.display(), "hello_world_dest");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(playground.path().join("hello_world_dest").exists());
    Ok(())
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
fn mv_files_with_glob_metachars_when_input_are_variables(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    playground.file(src_name, "What is the sound of one hand clapping?")?;

    let src = playground.path().join(src_name);

    let code = format!("let f = '{}'; mv $f {}", src.display(), "hello_world_dest");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(playground.path().join("hello_world_dest").exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_with_cd(playground: Playground) -> Result {
    playground.file("tmp_dir/file.txt", "body")?;

    let code = "do { cd tmp_dir; let f = 'file.txt'; mv $f .. }; open file.txt";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.stdout.trim(), "body");
    Ok(())
}

#[test]
#[deps(NU)]
fn test_mv_inside_glob_metachars_dir(playground: Playground) -> Result {
    let sub_dir = "test[]";
    playground.file(format!("{sub_dir}/test_file.txt"), "hello")?;

    let code = "mv test_file.txt ../";
    let result: CompleteResult = test()
        .cwd(playground.path().join(sub_dir))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(
        !playground
            .path()
            .join(sub_dir)
            .join("test_file.txt")
            .exists()
    );
    assert!(playground.path().join("test_file.txt").exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn test_mv_wildcards(playground: Playground) -> Result {
    let sub_dir = "test[]";
    playground.file(format!("{sub_dir}/.a"), "hello")?;

    let code = "mv * ../";
    let result: CompleteResult = test()
        .cwd(playground.path().join(sub_dir))
        .run_with_data(RUNNER, code)?;
    // by default, wildcard don't match dot files.
    assert_contains("File not found", result.stderr);
    assert!(playground.path().join(sub_dir).join(".a").exists());
    assert!(!playground.path().join(".a").exists());

    // unless `-a` flag is provided.
    let code = "mv -a * ../";
    let result: CompleteResult = test()
        .cwd(playground.path().join(sub_dir))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(!playground.path().join(sub_dir).join(".a").exists());
    assert!(playground.path().join(".a").exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn mv_with_tilde(playground: Playground) -> Result {
    playground.empty_file("~tilde/f1.txt")?;
    playground.empty_file("~tilde/f2.txt")?;
    playground.empty_file("~tilde/f3.txt")?;
    playground.dir("~tilde2")?;

    // mv file
    let code = "mv '~tilde/f1.txt' ./";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(!playground.path().join("~tilde/f1.txt").exists());
    assert!(playground.path().join("f1.txt").exists());

    // pass variable
    let code = "let f = '~tilde/f2.txt'; mv $f ./";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(!playground.path().join("~tilde/f2.txt").exists());
    assert!(playground.path().join("f1.txt").exists());
    Ok(())
}

#[test]
#[deps(NU)]
#[cfg_attr(target_os = "macos", ignore)]
fn mv_verbose_message_mentions_source_and_destination(playground: Playground) -> Result {
    playground.empty_file("before.txt")?;

    let code = "mv -v before.txt after.txt | table -w 200";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert_contains("before.txt", &result.stdout);
    assert_contains("after.txt", &result.stdout);
    assert!(playground.path().join("after.txt").exists());
    assert!(!playground.path().join("before.txt").exists());
    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
#[deps(NU)]
fn mv_literal_directory(playground: Playground) -> Result {
    playground.empty_file("subdir/test.txt")?;
    playground.dir("dest")?;

    let code = "mv subdir dest";
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(!playground.path().join("subdir").exists());
    assert!(playground.path().join("dest/subdir/test.txt").exists());
    Ok(())
}
