use std::{
    fs,
    hash::{BuildHasher, RandomState},
    path::{MAIN_SEPARATOR, Path},
    sync::LazyLock,
};

use nu_test_support::prelude::*;

use rstest::rstest;

static HASHER: LazyLock<RandomState> = LazyLock::new(RandomState::new);
fn file_hash(file: impl AsRef<Path>) -> Result<u64> {
    let content = fs::read_to_string(file)?;
    Ok(HASHER.hash_one(content))
}

const RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_a_file(#[ignore] playground: Playground, #[case] progress_flag: &str) -> Result {
    let test_file = FIXTURES.join("formats").join("sample.ini");
    // Get the hash of the file content to check integrity after copy.
    let first_hash = file_hash(&test_file)?;

    let code = format!("cp {progress_flag} `{}` sample.ini", test_file.display());
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(playground.path().join("sample.ini").exists());

    // Get the hash of the copied file content to check against first_hash.
    let after_cp_hash = file_hash(playground.path().join("sample.ini"))?;
    assert_eq!(first_hash, after_cp_hash);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    let expected_file = playground.path().join("sample.ini");
    // Get the hash of the file content to check integrity after copy.
    let formats = FIXTURES.join("formats");
    let first_hash = file_hash(formats.join("sample.ini"))?;
    let code = format!(
        "cp {progress_flag} sample.ini {}",
        playground.path().display(),
    );
    let result: CompleteResult = test().cwd(formats).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(expected_file.exists());

    // Check the integrity of the file.
    let after_cp_hash = file_hash(expected_file)?;
    assert_eq!(first_hash, after_cp_hash);
    Ok(())
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn error_if_attempting_to_copy_a_directory_to_another_directory(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    let formats = FIXTURES.join("formats");
    let code = format!("cp {progress_flag} . {}", playground.path().display());
    let result: CompleteResult = test().cwd(formats).run_with_data(RUNNER, code)?;

    assert_ne!(result.exit_code, 0);
    assert_contains("resolves to a directory (not copied)", result.stderr);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("originals", |originals| {
        originals.empty_file("yehuda.txt")?;
        originals.empty_file("jttxt")?;
        originals.empty_file("andres.txt")
    })?;
    playground.dir("expected")?;

    let expected_dir = playground.path().join("expected").join("originals");
    let code = format!("cp {progress_flag} originals expected -r");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(expected_dir.exists());
    assert!(expected_dir.join("yehuda.txt").exists());
    assert!(expected_dir.join("jttxt").exists());
    assert!(expected_dir.join("andres.txt").exists());
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn deep_copies_with_recursive_flag(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("originals", |originals| {
        originals.empty_file("manifest.txt")?;
        originals.at("contributors", |contributors| {
            contributors.empty_file("yehuda.txt")?;
            contributors.empty_file("jttxt")?;
            contributors.empty_file("andres.txt")?;
            contributors.empty_file("JT/errors.txt")?;
            contributors.empty_file("JT/multishells.txt")?;
            contributors.empty_file("andres/coverage.txt")?;
            contributors.empty_file("andres/commands.txt")?;
            contributors.empty_file("yehuda/defer-evaluation.txt")
        })
    })?;
    playground.dir("expected")?;

    let expected_dir = playground.path().join("expected").join("originals");
    let jts_expected_copied_dir = expected_dir.join("contributors").join("JT");
    let andres_expected_copied_dir = expected_dir.join("contributors").join("andres");
    let yehudas_expected_copied_dir = expected_dir.join("contributors").join("yehuda");

    let code = format!("cp {progress_flag} originals expected --recursive");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(expected_dir.exists());
    assert!(jts_expected_copied_dir.join("errors.txt").exists());
    assert!(jts_expected_copied_dir.join("multishells.txt").exists());
    assert!(andres_expected_copied_dir.join("coverage.txt").exists());
    assert!(andres_expected_copied_dir.join("commands.txt").exists());
    assert!(
        yehudas_expected_copied_dir
            .join("defer-evaluation.txt")
            .exists()
    );
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_using_path_with_wildcard(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    // Get the hash of the file content to check integrity after copy.
    let src_hashes: Vec<String> = test()
        .cwd(FIXTURES.join("formats"))
        .run("ls ../formats/* | where type == file | each { |file| open --raw $file.name | to text | hash md5 }")?;

    let code = format!(
        "cp {progress_flag} -r {}/* {}",
        FIXTURES.join("formats").display(),
        playground.path().display()
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    for file in [
        "caco3_plastics.csv",
        "cargo_sample.toml",
        "jt.xml",
        "sample.ini",
        "sgml_description.json",
        "utf16.ini",
    ] {
        assert!(playground.path().join(file).exists());
    }

    // Check integrity after the copy is done
    let dst_hashes: Vec<String> = test().cwd(playground.path()).run(format!(
        "
            ls {}
            | where type == file
            | each {{ |file|
                open --raw $file.name
                | to text
                | hash md5
            }}
        ",
        playground.path().display()
    ))?;
    assert_eq!(src_hashes, dst_hashes);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_using_a_glob(#[ignore] playground: Playground, #[case] progress_flag: &str) -> Result {
    // Get the hash of the file content to check integrity after copy.
    let src_hashes: Vec<String> = test().cwd(FIXTURES.join("formats")).run(
        "ls * | where type == file | each { |file| open --raw $file.name | to text | hash md5 }",
    )?;

    let code = format!("cp {progress_flag} -r * {}", playground.path().display());
    let result: CompleteResult = test()
        .cwd(FIXTURES.join("formats"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    for file in [
        "caco3_plastics.csv",
        "cargo_sample.toml",
        "jt.xml",
        "sample.ini",
        "sgml_description.json",
        "utf16.ini",
    ] {
        assert!(playground.path().join(file).exists());
    }

    // Check integrity after the copy is done
    let dst_hashes: Vec<String> = test().cwd(playground.path()).run(format!(
        "
            ls {}
            | where type == file
            | each {{ |file|
                open --raw $file.name
                | to text
                | hash md5
            }}
        ",
        playground.path().display()
    ))?;
    assert_eq!(src_hashes, dst_hashes);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_same_file_twice(#[ignore] playground: Playground, #[case] progress_flag: &str) -> Result {
    let code = format!(
        "cp {progress_flag} `{}` sample.ini",
        FIXTURES.join("formats").join("sample.ini").display()
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let code = format!(
        "cp {progress_flag} `{}` sample.ini",
        FIXTURES.join("formats").join("sample.ini").display()
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    assert!(playground.path().join("sample.ini").exists());
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore = "Behavior not supported by uutils cp"]
#[nu_test_support::test]
#[deps(NU)]
fn copy_files_using_glob_two_parents_up_using_multiple_dots(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("foo/bar", |bar| {
        bar.empty_file("jtjson")?;
        bar.empty_file("andres.xml")?;
        bar.empty_file("yehuda.yaml")?;
        bar.empty_file("kevin.txt")?;
        bar.empty_file("many_more.ppl")
    })?;

    let code = format!("cp {progress_flag} * ...");
    let result: CompleteResult = test()
        .cwd(playground.path().join("foo/bar"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    for file in [
        "yehuda.yaml",
        "jtjson",
        "andres.xml",
        "kevin.txt",
        "many_more.ppl",
    ] {
        assert!(playground.path().join(file).exists());
    }
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("hello_there")?;
    playground.dir("hello_again")?;
    playground.dir("foo/bar")?;

    let code = format!("cp {progress_flag} -r .../hello* .");
    let result: CompleteResult = test()
        .cwd(playground.path().join("foo/bar"))
        .run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let expected = playground.path().join("foo/bar");

    assert!(expected.join("hello_there").exists());
    assert!(expected.join("hello_again").exists());
    Ok(())
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_to_non_existing_dir(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("empty_file")?;

    let code = format!("cp {progress_flag} empty_file ~/not_a_dir{MAIN_SEPARATOR}");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_ne!(result.exit_code, 0);
    assert_contains("is not a directory", result.stderr);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_contains_symlink_ignored(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("tmp_dir", |tmp_dir| {
        tmp_dir.empty_file("hello_there")?;
        tmp_dir.empty_file("good_bye")?;
        tmp_dir.symlink("good_bye", "dangle_symlink")
    })?;

    // make symbolic link and copy.
    let code = format!("rm tmp_dir/good_bye; cp {progress_flag} -r tmp_dir tmp_dir_2");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` don't exists inside `tmp_dir_2`.
    let expected = playground.path().join("tmp_dir_2");
    assert!(expected.join("hello_there").exists());
    // GNU cp will copy the broken symlink, so following their behavior
    // thus commenting out below
    // let path = expected.join("dangle_symlink");
    // assert!(!path.exists() && !path.is_symlink());
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_contains_symlink(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("tmp_dir", |tmp_dir| {
        tmp_dir.empty_file("hello_there")?;
        tmp_dir.empty_file("good_bye")?;
        tmp_dir.symlink("good_bye", "dangle_symlink")
    })?;

    // make symbolic link and copy.
    let code = format!("rm tmp_dir/good_bye; cp {progress_flag} -r -n tmp_dir tmp_dir_2");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` also exists inside `tmp_dir_2`.
    let expected = playground.path().join("tmp_dir_2");
    assert!(expected.join("hello_there").exists());
    let path = expected.join("dangle_symlink");
    assert!(path.is_symlink());
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_symlink_file_body_not_changed(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.at("tmp_dir", |tmp_dir| {
        tmp_dir.empty_file("hello_there")?;
        tmp_dir.empty_file("good_bye")?;
        tmp_dir.symlink("good_bye", "dangle_symlink")
    })?;

    // make symbolic link and copy.
    let code = format!(
        "
                rm tmp_dir/good_bye
                cp {progress_flag} -r -n tmp_dir tmp_dir_2
                rm -r tmp_dir
                cp {progress_flag} -r -n tmp_dir_2 tmp_dir
                'hello_data' | save tmp_dir/good_bye
            "
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    // check dangle_symlink in tmp_dir is no longer dangling.
    let expected_file = playground.path().join("tmp_dir").join("dangle_symlink");
    let actual = fs::read_to_string(expected_file)?;
    assert_contains("hello_data", actual);
    Ok(())
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_identical_file(#[ignore] playground: Playground, #[case] progress_flag: &str) -> Result {
    playground.empty_file("same.txt")?;

    let code = format!("cp {progress_flag} same.txt same.txt");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;

    let msg = format!(
        "'{}' and '{}' are the same file",
        playground.path().join("same.txt").display(),
        playground.path().join("same.txt").display(),
    );
    // debug messages in CI
    if !result.stderr.contains(&msg) {
        panic!("stderr was: {}", result.stderr);
    }
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore = "File name in progress bar not on uutils impl"]
#[nu_test_support::test]
#[deps(NU)]
fn copy_ignores_ansi(#[ignore] playground: Playground, #[case] progress_flag: &str) -> Result {
    playground.empty_file("test.txt")?;

    let code = format!(
        "ls | find test | get name | cp {progress_flag} $in.0 success.txt; ls | find success | get name | ansi strip | get 0"
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout.trim(), "success.txt");
    Ok(())
}

//apparently on windows error msg is different, but linux(where i test) is fine.
//fix later FIXME
#[cfg(unix)]
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_not_exists_dst(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("valid.txt")?;
    let source = playground.path().join("valid.txt");
    let target = playground.path().join("invalid_dir").join("invalid_dir1");

    let code = format!(
        "cp {progress_flag} {source} {target}",
        source = source.display(),
        target = target.display()
    );
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_contains("invalid_dir1", &result.stderr);
    assert_contains("No such file or directory", &result.stderr);
    Ok(())
}

//again slightly different error message on windows on tests
// compared to linux
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore] //FIXME: This test needs to be re-enabled once uu_cp has fixed the bug
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_with_read_permission(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("valid.txt")?;
    playground.readonly_file("invalid_prem.txt", "")?;

    let code = format!("cp {progress_flag} valid.txt invalid_prem.txt");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_ne!(result.exit_code, 0);
    assert_contains("invalid_prem.txt", &result.stderr);
    assert_contains("denied", &result.stderr);
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_file_symlink_without_dereferencing(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("file")?;
    playground.symlink("file", "link_to_file")?;

    let code = format!("cp {progress_flag} --no-dereference link_to_file second_link_to_file");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let second_link = playground.path().join("second_link_to_file");
    assert!(second_link.is_symlink());
    assert_eq!(fs::read_link(second_link)?, playground.path().join("file"));
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_directory_symlink_without_dereferencing(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.dir("dir")?;
    playground.symlink("dir", "link_to_dir")?;

    let code = format!("cp {progress_flag} -P link_to_dir second_link_to_dir");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

    let second_link = playground.path().join("second_link_to_dir");
    assert!(second_link.is_symlink());
    assert_eq!(fs::read_link(second_link)?, playground.path().join("dir"));
    Ok(())
}

// uutils/coreutils copy tests
static TEST_EXISTING_FILE: &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";

#[test]
fn test_cp_cp(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);

    // Get the hash of the file content to check integrity after copy.
    let src_hash = file_hash(&src)?;

    let () = test()
        .cwd(playground.path())
        .run(format!("cp {} {TEST_HELLO_WORLD_DEST}", src.display(),))?;

    assert!(playground.path().join(TEST_HELLO_WORLD_DEST).exists());

    // Get the hash of the copied file content to check against first_hash.
    let after_cp_hash = file_hash(playground.path().join(TEST_HELLO_WORLD_DEST))?;
    assert_eq!(src_hash, after_cp_hash);
    Ok(())
}

#[test]
#[serial]
fn test_cp_existing_target(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let existing = FIXTURES.join("cp").join(TEST_EXISTING_FILE);

    // Get the hash of the file content to check integrity after copy.
    let src_hash = file_hash(&src)?;

    // Copy existing file to destination, so that it exists for the test
    let () = test()
        .cwd(playground.path())
        .run(format!("cp {} {TEST_EXISTING_FILE}", existing.display(),))?;

    // At this point the src and existing files should be different
    assert!(playground.path().join(TEST_EXISTING_FILE).exists());

    // Now for the test
    let () = test()
        .cwd(playground.path())
        .run(format!("cp {} {TEST_EXISTING_FILE}", src.display(),))?;

    assert!(playground.path().join(TEST_EXISTING_FILE).exists());

    // Get the hash of the copied file content to check against first_hash.
    let after_cp_hash = file_hash(playground.path().join(TEST_EXISTING_FILE))?;
    assert_eq!(src_hash, after_cp_hash);
    Ok(())
}

#[test]
fn test_cp_multiple_files(playground: Playground) -> Result {
    let src1 = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let src2 = FIXTURES.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);

    // Get the hash of the file content to check integrity after copy.
    let src1_hash = file_hash(&src1)?;
    let src2_hash = file_hash(&src2)?;

    //Create target directory
    playground.dir(TEST_COPY_TO_FOLDER)?;

    // Start test
    let () = test().cwd(playground.path()).run(format!(
        "cp {} {} {TEST_COPY_TO_FOLDER}",
        src1.display(),
        src2.display(),
    ))?;

    assert!(playground.path().join(TEST_COPY_TO_FOLDER).exists());

    // Get the hash of the copied file content to check against first_hash.
    let after_cp_1_hash = file_hash(playground.path().join(TEST_COPY_TO_FOLDER_FILE))?;
    let after_cp_2_hash = file_hash(playground.path().join(TEST_HOW_ARE_YOU_DEST))?;
    assert_eq!(src1_hash, after_cp_1_hash);
    assert_eq!(src2_hash, after_cp_2_hash);
    Ok(())
}

#[test]
fn test_cp_recurse(playground: Playground) -> Result {
    // Create the relevant target directories
    playground.dir(TEST_COPY_FROM_FOLDER)?;
    playground.dir(TEST_COPY_TO_FOLDER_NEW)?;
    let src = FIXTURES.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);

    let src_hash = file_hash(src)?;
    // Start test
    let () = test().cwd(FIXTURES.join("cp")).run(format!(
        "cp -r {TEST_COPY_FROM_FOLDER}* {}",
        playground.path().join(TEST_COPY_TO_FOLDER_NEW).display()
    ))?;
    let after_cp_hash = file_hash(playground.path().join(TEST_COPY_TO_FOLDER_NEW_FILE))?;
    assert_eq!(src_hash, after_cp_hash);
    Ok(())
}

#[test]
fn test_cp_with_dirs(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let src_hash = file_hash(&src)?;

    //Create target directory
    playground.dir(TEST_COPY_TO_FOLDER)?;
    // Start test
    let () = test()
        .cwd(playground.path())
        .run(format!("cp {} {TEST_COPY_TO_FOLDER}", src.display(),))?;
    let after_cp_hash = file_hash(playground.path().join(TEST_COPY_TO_FOLDER_FILE))?;
    assert_eq!(src_hash, after_cp_hash);

    // Other way around
    playground.dir(TEST_COPY_FROM_FOLDER)?;
    let src2 = FIXTURES.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
    let src2_hash = file_hash(&src2)?;
    let () = test()
        .cwd(playground.path())
        .run(format!("cp {} {TEST_HELLO_WORLD_DEST}", src2.display(),))?;
    let after_cp_2_hash = file_hash(playground.path().join(TEST_HELLO_WORLD_DEST))?;
    assert_eq!(src2_hash, after_cp_2_hash);
    Ok(())
}
#[cfg(not(windows))]
#[test]
fn test_cp_arg_force(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let src_hash = file_hash(&src)?;
    playground.readonly_file("invalid_prem.txt", "")?;

    let () = test().cwd(playground.path()).run(format!(
        "cp {} --force {}",
        src.display(),
        "invalid_prem.txt"
    ))?;
    let after_cp_hash = file_hash(playground.path().join("invalid_prem.txt"))?;
    // Check content was copied by the use of --force
    assert_eq!(src_hash, after_cp_hash);
    Ok(())
}

#[test]
#[deps(NU)]
fn test_cp_directory_to_itself_disallowed(playground: Playground) -> Result {
    playground.dir("d")?;
    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data(RUNNER, format!("cp -r {} {}", "d", "d"))?;
    assert_contains("cannot copy a directory", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn test_cp_nested_directory_to_itself_disallowed(playground: Playground) -> Result {
    playground.dir("a")?;
    playground.dir("a/b")?;
    playground.dir("a/b/c")?;
    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data(RUNNER, format!("cp -r {} {}", "a/b", "a/b/c"))?;
    assert_contains("cannot copy a directory", result.stderr);
    Ok(())
}

#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn test_cp_same_file_force(playground: Playground) -> Result {
    playground.empty_file("f")?;
    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data(RUNNER, format!("cp --force {} {}", "f", "f"))?;
    let path = playground.path().join("f");
    assert_contains(
        format!(
            "'{}' and '{}' are the same file",
            path.display(),
            path.display()
        ),
        result.stderr,
    );
    assert!(!playground.path().join("f~").exists());
    Ok(())
}

#[test]
#[serial]
fn test_cp_arg_no_clobber(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let target = FIXTURES.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
    let target_hash = file_hash(&target)?;

    let () = test().cwd(playground.path()).run(format!(
        "cp {} {} --no-clobber",
        src.display(),
        target.display()
    ))?;
    let after_cp_hash = file_hash(target)?;
    // Check content was not clobbered
    assert_eq!(after_cp_hash, target_hash);
    Ok(())
}

#[test]
#[serial]
fn test_cp_arg_no_clobber_twice(playground: Playground) -> Result {
    playground.file("source.txt", "fake data")?;
    playground.file("source_with_body.txt", "some-body")?;
    let () = test()
        .cwd(playground.path())
        .run(format!("cp --no-clobber {} {}", "source.txt", "dest.txt"))?;
    assert!(playground.path().join("dest.txt").exists());

    let () = test().cwd(playground.path()).run(format!(
        "cp --no-clobber {} {}",
        "source_with_body.txt", "dest.txt"
    ))?;
    // Should have same contents of original empty file as --no-clobber should not overwrite dest.txt
    assert_eq!(
        fs::read_to_string(playground.path().join("dest.txt"))?,
        "fake data"
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn test_cp_debug_default(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);

    let actual: CompleteResult = test().cwd(playground.path()).run_with_data(
        RUNNER,
        format!("cp --debug `{}` {TEST_HELLO_WORLD_DEST}", src.display()),
    )?;

    #[cfg(target_os = "macos")]
    if !actual
        .stdout
        .contains("copy offload: unknown, reflink: unsupported, sparse detection: unsupported")
    {
        panic!("Failure: stdout was \n{}", actual.stdout);
    }

    #[cfg(target_os = "linux")]
    if !actual
        .stdout
        .contains("copy offload: yes, reflink: unsupported, sparse detection: no")
    {
        panic!("Failure: stdout was \n{}", actual.stdout);
    }

    #[cfg(target_os = "freebsd")]
    if !actual
        .stdout
        .contains("copy offload: unsupported, reflink: unsupported, sparse detection: unsupported")
    {
        panic!("Failure: stdout was \n{}", actual.stdout);
    }

    #[cfg(windows)]
    if !actual
        .stdout
        .contains("copy offload: unsupported, reflink: unsupported, sparse detection: unsupported")
    {
        panic!("Failure: stdout was \n{}", actual.stdout);
    }
    Ok(())
}

#[test]
#[deps(NU)]
fn test_cp_verbose_default(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);

    let actual: CompleteResult = test().cwd(playground.path()).run_with_data(
        RUNNER,
        format!("cp --verbose `{}` {TEST_HELLO_WORLD_DEST}", src.display()),
    )?;
    assert_contains(
        format!(
            "'{}' -> '{}'",
            src.display(),
            playground.path().join(TEST_HELLO_WORLD_DEST).display()
        ),
        actual.stdout,
    );
    Ok(())
}

#[test]
fn test_cp_only_source_no_dest(playground: Playground) -> Result {
    let src = FIXTURES.join("cp").join(TEST_HELLO_WORLD_SOURCE);
    let err = test()
        .cwd(playground.path())
        .run(format!("cp {}", src.display(),))
        .expect_shell_error()?;
    let msg = err.generic_msg()?;
    assert_contains("Missing destination path operand after", &msg);
    assert_contains(TEST_HELLO_WORLD_SOURCE, &msg);
    Ok(())
}

#[test]
fn test_cp_with_vars(playground: Playground) -> Result {
    playground.empty_file("input")?;
    let () = test()
        .cwd(playground.path())
        .run("let src = 'input'; let dst = 'target'; cp $src $dst")?;
    assert!(playground.path().join("target").exists());
    Ok(())
}

#[test]
fn test_cp_destination_after_cd(playground: Playground) -> Result {
    playground.dir("test")?;
    playground.empty_file("test/file.txt")?;
    let () = test().cwd(playground.path()).run(
        // Defining variable avoid path expansion of cp argument.
        // If argument was not expanded ucp wrapper should do it
        "cd test; let file = 'copy.txt'; cp file.txt $file",
    )?;
    assert!(playground.path().join("test").join("copy.txt").exists());
    Ok(())
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg_attr(windows, ignore)]
#[case("'a]?c'")]
#[cfg_attr(windows, ignore)]
#[case("'a*.?c'")]
fn copies_files_with_glob_metachars(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    playground.file(src_name, "What is the sound of one hand clapping?")?;

    let src = playground.path().join(src_name);

    let () = test()
        .cwd(playground.path())
        .run(format!("cp '{}' {TEST_HELLO_WORLD_DEST}", src.display(),))?;

    assert!(playground.path().join(TEST_HELLO_WORLD_DEST).exists());
    Ok(())
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg_attr(windows, ignore)]
#[case("'a]?c'")]
#[cfg_attr(windows, ignore)]
#[case("'a*.?c'")]
fn copies_files_with_glob_metachars_when_input_are_variables(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    playground.file(src_name, "What is the sound of one hand clapping?")?;

    let src = playground.path().join(src_name);

    let () = test().cwd(playground.path()).run(format!(
        "let f = '{}'; cp $f {TEST_HELLO_WORLD_DEST}",
        src.display(),
    ))?;

    assert!(playground.path().join(TEST_HELLO_WORLD_DEST).exists());
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_timestamps(playground: Playground) -> Result {
    // Preserve timestamp and mode

    playground.empty_file("file.txt")?;
    let code = "
        chmod +x file.txt
        cp --preserve [ mode timestamps ] file.txt other.txt
        
        let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
        let new_attrs = ls -l other.txt | get 0 | select mode accessed modified
        
        $old_attrs == $new_attrs
    ";

    test()
        .cwd(playground.path())
        .inherit_path()
        .run(code)
        .expect_value_eq(true)
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_only_timestamps(playground: Playground) -> Result {
    // Preserve timestamps and discard all other attributes including mode

    playground.empty_file("file.txt")?;
    let code = "
        chmod +x file.txt
        cp --preserve [ timestamps ] file.txt other.txt
        
        let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
        let new_attrs = ls -l other.txt | get 0 | select mode accessed modified
        
        [
            (($old_attrs | select mode) != ($new_attrs | select mode)),
            (($old_attrs | select accessed modified) == ($new_attrs | select accessed modified)),
        ]
    ";

    test()
        .cwd(playground.path())
        .inherit_path()
        .run(code)
        .expect_value_eq([true, true])
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_nothing(playground: Playground) -> Result {
    // Preserve no attributes

    playground.empty_file("file.txt")?;
    let code = "
        chmod +x file.txt
        cp --preserve [] file.txt other.txt
        
        let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
        let new_attrs = ls -l other.txt | get 0 | select mode accessed modified
        
        $old_attrs != $new_attrs
    ";

    test()
        .cwd(playground.path())
        .inherit_path()
        .run(code)
        .expect_value_eq(true)
}

#[test]
fn test_cp_inside_glob_metachars_dir(playground: Playground) -> Result {
    let sub_dir = "test[]";
    playground.file("test[]/test_file.txt", "hello")?;

    let () = test()
        .cwd(playground.path().join(sub_dir))
        .run("cp test_file.txt ../")?;

    assert!(
        playground
            .path()
            .join(sub_dir)
            .join("test_file.txt")
            .exists()
    );
    assert!(playground.path().join("test_file.txt").exists());
    Ok(())
}

#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn test_cp_to_customized_home_directory(playground: Playground) -> Result {
    playground.empty_file("test_file.txt")?;
    let code = "mkdir test; cp test_file.txt ~/test/";
    let result: CompleteResult = test()
        .cwd(playground.path())
        .env("HOME", playground.path())
        .run_with_data("nu -n -c $in | complete", code)?;

    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert!(playground.path().join("test/test_file.txt").exists());
    Ok(())
}

#[test]
fn cp_with_tilde(playground: Playground) -> Result {
    playground.at("~tilde", |tilde| {
        tilde.empty_file("f1.txt")?;
        tilde.empty_file("f2.txt")?;
        tilde.empty_file("f3.txt")
    })?;
    playground.dir("~tilde2")?;
    // cp directory
    test()
        .cwd(playground.path())
        .run("let f = '~tilde'; cp -r $f '~tilde2'; ls '~tilde2/~tilde' | length")
        .expect_value_eq(3)?;

    // cp file
    let () = test().cwd(playground.path()).run("cp '~tilde/f1.txt' ./")?;
    assert!(playground.path().join("~tilde/f1.txt").exists());
    assert!(playground.path().join("f1.txt").exists());

    // pass variable
    let () = test()
        .cwd(playground.path())
        .run("let f = '~tilde/f2.txt'; cp $f ./")?;
    assert!(playground.path().join("~tilde/f2.txt").exists());
    assert!(playground.path().join("f1.txt").exists());
    Ok(())
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_with_update_flag(
    #[ignore] playground: Playground,
    #[case] progress_flag: &str,
) -> Result {
    playground.empty_file("valid.txt")?;
    playground.file("newer_valid.txt", "body")?;

    let code = format!("cp {progress_flag} -u valid.txt newer_valid.txt; open newer_valid.txt");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout.trim(), "body");

    // create a file after assert to make sure that newest_valid.txt is newest
    std::thread::sleep(std::time::Duration::from_secs(1));
    playground.file("newest_valid.txt", "newest_body")?;
    let code = format!("cp {progress_flag} -u newest_valid.txt valid.txt; open valid.txt");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout.trim(), "newest_body");

    // when destination doesn't exist
    let code =
        format!("cp {progress_flag} -u newest_valid.txt des_missing.txt; open des_missing.txt");
    let result: CompleteResult = test().cwd(playground.path()).run_with_data(RUNNER, code)?;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout.trim(), "newest_body");
    Ok(())
}

#[test]
fn cp_with_cd(playground: Playground) -> Result {
    playground.file("tmp_dir/file.txt", "body")?;

    test()
        .cwd(playground.path())
        .run("do { cd tmp_dir; let f = 'file.txt'; cp $f .. }; open file.txt")
        .expect_value_eq("body")
}

#[test]
fn test_cp_wildcards(playground: Playground) -> Result {
    let sub_dir = "test[]";
    playground.file("test[]/.a", "hello")?;

    let err = test()
        .cwd(playground.path().join(sub_dir))
        .run("cp * ../")
        .expect_shell_error()?;
    // by default, wildcard don't match dot files.
    assert_contains("FileNotFound", format!("{err:?}"));
    assert!(playground.path().join(sub_dir).join(".a").exists());
    assert!(!playground.path().join(".a").exists());

    // unless `-a` flag is provided.
    let () = test()
        .cwd(playground.path().join(sub_dir))
        .run("cp -a * ../")?;
    // by default, wildcard don't match dot files.
    assert!(playground.path().join(sub_dir).join(".a").exists());
    assert!(playground.path().join(".a").exists());
    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn cp_literal_directory_with_recursive_flag(playground: Playground) -> Result {
    playground.empty_file("subdir/test.txt")?;
    playground.dir("dest")?;

    let () = test()
        .cwd(playground.path())
        .run("cp subdir dest --recursive")?;

    assert!(playground.path().join("dest/subdir/test.txt").exists());
    Ok(())
}
