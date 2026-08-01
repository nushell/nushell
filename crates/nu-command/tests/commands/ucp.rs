use std::{
    fs,
    hash::{BuildHasher, RandomState},
    path::{MAIN_SEPARATOR, Path},
    sync::LazyLock,
};

use nu_test_support::fs::{
    Stub::{EmptyFile, FileWithContent, FileWithPermission},
    files_exist_at,
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
fn copies_a_file(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_1", |dirs, _| {
        let test_file = dirs.formats().join("sample.ini");
        // Get the hash of the file content to check integrity after copy.
        let first_hash = file_hash(&test_file)?;

        let code = format!(
            "cp {progress_flag} `{}` ucp_test_1/sample.ini",
            test_file.display()
        );
        let result: CompleteResult = test().cwd(dirs.root()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(dirs.test().join("sample.ini").exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = file_hash(dirs.test().join("sample.ini"))?;
        assert_eq!(first_hash, after_cp_hash);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory(
    #[case] progress_flag: &str,
) -> Result {
    Playground::setup("ucp_test_2", |dirs, _| {
        let expected_file = dirs.test().join("sample.ini");
        // Get the hash of the file content to check integrity after copy.
        let first_hash = file_hash(dirs.formats().join("../formats/sample.ini"))?;
        let code = format!(
            "cp {progress_flag} ../formats/sample.ini {}",
            dirs.test().display(),
        );
        let result: CompleteResult = test().cwd(dirs.formats()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(expected_file.exists());

        // Check the integrity of the file.
        let after_cp_hash = file_hash(expected_file)?;
        assert_eq!(first_hash, after_cp_hash);
        Ok(())
    })
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn error_if_attempting_to_copy_a_directory_to_another_directory(
    #[case] progress_flag: &str,
) -> Result {
    Playground::setup("ucp_test_3", |dirs, _| {
        let code = format!("cp {progress_flag} ../formats {}", dirs.test().display());
        let result: CompleteResult = test().cwd(dirs.formats()).run_with_data(RUNNER, code)?;

        assert_ne!(result.exit_code, 0);
        assert_contains("resolves to a directory (not copied)", result.stderr);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag(
    #[case] progress_flag: &str,
) -> Result {
    Playground::setup("ucp_test_4", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(&[
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");
        let code = format!("cp {progress_flag} originals expected -r");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(expected_dir.exists());
        assert!(expected_dir.join("yehuda.txt").exists());
        assert!(expected_dir.join("jttxt").exists());
        assert!(expected_dir.join("andres.txt").exists());
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn deep_copies_with_recursive_flag(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_5", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(&[EmptyFile("manifest.txt")])
            .within("originals/contributors")
            .with_files(&[
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ])
            .within("originals/contributors/JT")
            .with_files(&[EmptyFile("errors.txt"), EmptyFile("multishells.txt")])
            .within("originals/contributors/andres")
            .with_files(&[EmptyFile("coverage.txt"), EmptyFile("commands.txt")])
            .within("originals/contributors/yehuda")
            .with_files(&[EmptyFile("defer-evaluation.txt")])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");
        let jts_expected_copied_dir = expected_dir.join("contributors").join("JT");
        let andres_expected_copied_dir = expected_dir.join("contributors").join("andres");
        let yehudas_expected_copied_dir = expected_dir.join("contributors").join("yehuda");

        let code = format!("cp {progress_flag} originals expected --recursive");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            &["errors.txt", "multishells.txt"],
            jts_expected_copied_dir
        ));
        assert!(files_exist_at(
            &["coverage.txt", "commands.txt"],
            andres_expected_copied_dir
        ));
        assert!(files_exist_at(
            &["defer-evaluation.txt"],
            yehudas_expected_copied_dir
        ));
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_using_path_with_wildcard(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_6", |dirs, _| {
        // Get the hash of the file content to check integrity after copy.
        let src_hashes: Vec<String> = test()
            .cwd(dirs.formats())
            .run("ls ../formats/* | where type == file | each { |file| open --raw $file.name | to text | hash md5 }")?;

        let code = format!(
            "cp {progress_flag} -r ../formats/* {}",
            dirs.test().display()
        );
        let result: CompleteResult = test().cwd(dirs.formats()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(files_exist_at(
            &[
                "caco3_plastics.csv",
                "cargo_sample.toml",
                "jt.xml",
                "sample.ini",
                "sgml_description.json",
                "utf16.ini",
            ],
            dirs.test()
        ));

        // Check integrity after the copy is done
        let dst_hashes: Vec<String> = test().cwd(dirs.formats()).run(format!(
            "
                ls {}
                | where type == file
                | each {{ |file|
                    open --raw $file.name
                    | to text
                    | hash md5
                }}
            ",
            dirs.test().display()
        ))?;
        assert_eq!(src_hashes, dst_hashes);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_using_a_glob(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_7", |dirs, _| {
        // Get the hash of the file content to check integrity after copy.
        let src_hashes: Vec<String> = test()
            .cwd(dirs.formats())
            .run("ls * | where type == file | each { |file| open --raw $file.name | to text | hash md5 }")?;

        let code = format!("cp {progress_flag} -r * {}", dirs.test().display());
        let result: CompleteResult = test().cwd(dirs.formats()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(files_exist_at(
            &[
                "caco3_plastics.csv",
                "cargo_sample.toml",
                "jt.xml",
                "sample.ini",
                "sgml_description.json",
                "utf16.ini",
            ],
            dirs.test()
        ));

        // Check integrity after the copy is done
        let dst_hashes: Vec<String> = test().cwd(dirs.formats()).run(format!(
            "
                    ls {}
                    | where type == file
                    | each {{ |file|
                        open --raw $file.name
                        | to text
                        | hash md5
                    }}
                ",
            dirs.test().display()
        ))?;
        assert_eq!(src_hashes, dst_hashes);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_same_file_twice(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_8", |dirs, _| {
        let code = format!(
            "cp {progress_flag} `{}` ucp_test_8/sample.ini",
            dirs.formats().join("sample.ini").display()
        );
        let result: CompleteResult = test().cwd(dirs.root()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let code = format!(
            "cp {progress_flag} `{}` ucp_test_8/sample.ini",
            dirs.formats().join("sample.ini").display()
        );
        let result: CompleteResult = test().cwd(dirs.root()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(dirs.test().join("sample.ini").exists());
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore = "Behavior not supported by uutils cp"]
#[nu_test_support::test]
#[deps(NU)]
fn copy_files_using_glob_two_parents_up_using_multiple_dots(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_9", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(&[
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        let code = format!("cp {progress_flag} * ...");
        let result: CompleteResult = test()
            .cwd(dirs.test().join("foo/bar"))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        assert!(files_exist_at(
            &[
                "yehuda.yaml",
                "jtjson",
                "andres.xml",
                "kevin.txt",
                "many_more.ppl",
            ],
            dirs.test()
        ));
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive(
    #[case] progress_flag: &str,
) -> Result {
    Playground::setup("ucp_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("hello_there")]);
        sandbox.mkdir("hello_again");
        sandbox.within("foo").mkdir("bar");

        let code = format!("cp {progress_flag} -r .../hello* .");
        let result: CompleteResult = test()
            .cwd(dirs.test().join("foo/bar"))
            .run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let expected = dirs.test().join("foo/bar");

        assert!(files_exist_at(&["hello_there", "hello_again"], expected));
        Ok(())
    })
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_to_non_existing_dir(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_11", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("empty_file")]);

        let code = format!("cp {progress_flag} empty_file ~/not_a_dir{MAIN_SEPARATOR}");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_ne!(result.exit_code, 0);
        assert_contains("is not a directory", result.stderr);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_contains_symlink_ignored(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_12", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        // make symbolic link and copy.
        let code = format!("rm tmp_dir/good_bye; cp {progress_flag} -r tmp_dir tmp_dir_2");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` don't exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(&["hello_there"], expected));
        // GNU cp will copy the broken symlink, so following their behavior
        // thus commenting out below
        // let path = expected.join("dangle_symlink");
        // assert!(!path.exists() && !path.is_symlink());
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_contains_symlink(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_13", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        // make symbolic link and copy.
        let code = format!("rm tmp_dir/good_bye; cp {progress_flag} -r -n tmp_dir tmp_dir_2");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` also exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(&["hello_there"], expected.clone()));
        let path = expected.join("dangle_symlink");
        assert!(path.is_symlink());
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_dir_symlink_file_body_not_changed(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_14", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

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
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        // check dangle_symlink in tmp_dir is no longer dangling.
        let expected_file = sandbox.cwd().join("tmp_dir").join("dangle_symlink");
        let actual = fs::read_to_string(expected_file)?;
        assert_contains("hello_data", actual);
        Ok(())
    })
}

// error msg changes on coreutils
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_identical_file(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_15", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("same.txt")]);

        let code = format!("cp {progress_flag} same.txt same.txt");
        let result: CompleteResult = test().cwd(dirs.test()).run_with_data(RUNNER, code)?;

        let msg = format!(
            "'{}' and '{}' are the same file",
            dirs.test().join("same.txt").display(),
            dirs.test().join("same.txt").display(),
        );
        // debug messages in CI
        if !result.stderr.contains(&msg) {
            panic!("stderr was: {}", result.stderr);
        }
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore = "File name in progress bar not on uutils impl"]
#[nu_test_support::test]
#[deps(NU)]
fn copy_ignores_ansi(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_16", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        let code = format!(
            "ls | find test | get name | cp {progress_flag} $in.0 success.txt; ls | find success | get name | ansi strip | get 0"
        );
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert_eq!(result.stdout.trim(), "success.txt");
        Ok(())
    })
}

//apparently on windows error msg is different, but linux(where i test) is fine.
//fix later FIXME
#[cfg(unix)]
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_not_exists_dst(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_17", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("valid.txt")]);
        let source = dirs.test().join("valid.txt");
        let target = dirs.test().join("invalid_dir").join("invalid_dir1");

        let code = format!(
            "cp {progress_flag} {source} {target}",
            source = source.display(),
            target = target.display()
        );
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_contains("invalid_dir1", &result.stderr);
        assert_contains("No such file or directory", &result.stderr);
        Ok(())
    })
}

//again slightly different error message on windows on tests
// compared to linux
#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[ignore] //FIXME: This test needs to be re-enabled once uu_cp has fixed the bug
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_with_read_permission(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_test_18", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithPermission("invalid_prem.txt", false),
        ]);

        let code = format!("cp {progress_flag} valid.txt invalid_prem.txt");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_ne!(result.exit_code, 0);
        assert_contains("invalid_prem.txt", &result.stderr);
        assert_contains("denied", &result.stderr);
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_file_symlink_without_dereferencing(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_file_symlink_no_dereference", |_dirs, sandbox| {
        sandbox
            .with_files(&[EmptyFile("file")])
            .symlink("file", "link_to_file");

        let code = format!("cp {progress_flag} --no-dereference link_to_file second_link_to_file");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let second_link = sandbox.cwd().join("second_link_to_file");
        assert!(second_link.is_symlink());
        assert_eq!(fs::read_link(second_link)?, sandbox.cwd().join("file"));
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copies_directory_symlink_without_dereferencing(#[case] progress_flag: &str) -> Result {
    Playground::setup("ucp_dir_symlink_no_dereference", |_dirs, sandbox| {
        sandbox.mkdir("dir").symlink("dir", "link_to_dir");

        let code = format!("cp {progress_flag} -P link_to_dir second_link_to_dir");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);

        let second_link = sandbox.cwd().join("second_link_to_dir");
        assert!(second_link.is_symlink());
        assert_eq!(fs::read_link(second_link)?, sandbox.cwd().join("dir"));
        Ok(())
    })
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
fn test_cp_cp() -> Result {
    Playground::setup("ucp_test_19", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = file_hash(&src)?;

        let () = test().cwd(dirs.root()).run(format!(
            "cp {} ucp_test_19/{TEST_HELLO_WORLD_DEST}",
            src.display(),
        ))?;

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST))?;
        assert_eq!(src_hash, after_cp_hash);
        Ok(())
    })
}

#[test]
fn test_cp_existing_target() -> Result {
    Playground::setup("ucp_test_20", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let existing = dirs.fixtures.join("cp").join(TEST_EXISTING_FILE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = file_hash(&src)?;

        // Copy existing file to destination, so that it exists for the test
        let () = test().cwd(dirs.root()).run(format!(
            "cp {} ucp_test_20/{TEST_EXISTING_FILE}",
            existing.display(),
        ))?;

        // At this point the src and existing files should be different
        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Now for the test
        let () = test().cwd(dirs.root()).run(format!(
            "cp {} ucp_test_20/{TEST_EXISTING_FILE}",
            src.display(),
        ))?;

        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = file_hash(dirs.test().join(TEST_EXISTING_FILE))?;
        assert_eq!(src_hash, after_cp_hash);
        Ok(())
    })
}

#[test]
fn test_cp_multiple_files() -> Result {
    Playground::setup("ucp_test_21", |dirs, sandbox| {
        let src1 = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src2 = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src1_hash = file_hash(&src1)?;
        let src2_hash = file_hash(&src2)?;

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);

        // Start test
        let () = test().cwd(dirs.root()).run(format!(
            "cp {} {} ucp_test_21/{TEST_COPY_TO_FOLDER}",
            src1.display(),
            src2.display(),
        ))?;

        assert!(dirs.test().join(TEST_COPY_TO_FOLDER).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_1_hash = file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE))?;
        let after_cp_2_hash = file_hash(dirs.test().join(TEST_HOW_ARE_YOU_DEST))?;
        assert_eq!(src1_hash, after_cp_1_hash);
        assert_eq!(src2_hash, after_cp_2_hash);
        Ok(())
    })
}

#[test]
fn test_cp_recurse() -> Result {
    Playground::setup("ucp_test_22", |dirs, sandbox| {
        // Create the relevant target directories
        sandbox.mkdir(TEST_COPY_FROM_FOLDER);
        sandbox.mkdir(TEST_COPY_TO_FOLDER_NEW);
        let src = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);

        let src_hash = file_hash(src)?;
        // Start test
        let () = test().cwd(dirs.fixtures.join("cp")).run(format!(
            "cp -r {TEST_COPY_FROM_FOLDER}* {}",
            dirs.test().join(TEST_COPY_TO_FOLDER_NEW).display()
        ))?;
        let after_cp_hash = file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_NEW_FILE))?;
        assert_eq!(src_hash, after_cp_hash);
        Ok(())
    })
}

#[test]
fn test_cp_with_dirs() -> Result {
    Playground::setup("ucp_test_23", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = file_hash(&src)?;

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        let () = test().cwd(dirs.root()).run(format!(
            "cp {} ucp_test_23/{TEST_COPY_TO_FOLDER}",
            src.display(),
        ))?;
        let after_cp_hash = file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE))?;
        assert_eq!(src_hash, after_cp_hash);

        // Other way around
        sandbox.mkdir(TEST_COPY_FROM_FOLDER);
        let src2 = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src2_hash = file_hash(&src2)?;
        let () = test().cwd(dirs.root()).run(format!(
            "cp {} ucp_test_23/{TEST_HELLO_WORLD_DEST}",
            src2.display(),
        ))?;
        let after_cp_2_hash = file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST))?;
        assert_eq!(src2_hash, after_cp_2_hash);
        Ok(())
    })
}
#[cfg(not(windows))]
#[test]
fn test_cp_arg_force() -> Result {
    Playground::setup("ucp_test_24", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = file_hash(&src)?;
        sandbox.with_files(&[FileWithPermission("invalid_prem.txt", false)]);

        let () = test().cwd(dirs.root()).run(format!(
            "cp {} --force ucp_test_24/{}",
            src.display(),
            "invalid_prem.txt"
        ))?;
        let after_cp_hash = file_hash(dirs.test().join("invalid_prem.txt"))?;
        // Check content was copied by the use of --force
        assert_eq!(src_hash, after_cp_hash);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_cp_directory_to_itself_disallowed() -> Result {
    Playground::setup("ucp_test_25", |dirs, sandbox| {
        sandbox.mkdir("d");
        let result: CompleteResult = test().cwd(dirs.root()).run_with_data(
            RUNNER,
            format!("cp -r ucp_test_25/{}  ucp_test_25/{}", "d", "d"),
        )?;
        assert_contains("cannot copy a directory", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_cp_nested_directory_to_itself_disallowed() -> Result {
    Playground::setup("ucp_test_26", |dirs, sandbox| {
        sandbox.mkdir("a");
        sandbox.mkdir("a/b");
        sandbox.mkdir("a/b/c");
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run_with_data(RUNNER, format!("cp -r {} {}", "a/b", "a/b/c"))?;
        assert_contains("cannot copy a directory", result.stderr);
        Ok(())
    })
}

#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn test_cp_same_file_force() -> Result {
    Playground::setup("ucp_test_27", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("f")]);
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run_with_data(RUNNER, format!("cp --force {} {}", "f", "f"))?;
        let path = dirs.test().join("f");
        assert_contains(
            format!(
                "'{}' and '{}' are the same file",
                path.display(),
                path.display()
            ),
            result.stderr,
        );
        assert!(!dirs.test().join("f~").exists());
        Ok(())
    })
}

#[test]
fn test_cp_arg_no_clobber() -> Result {
    Playground::setup("ucp_test_28", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let target = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let target_hash = file_hash(&target)?;

        let () = test().cwd(dirs.root()).run(format!(
            "cp {} {} --no-clobber",
            src.display(),
            target.display()
        ))?;
        let after_cp_hash = file_hash(target)?;
        // Check content was not clobbered
        assert_eq!(after_cp_hash, target_hash);
        Ok(())
    })
}

#[test]
fn test_cp_arg_no_clobber_twice() -> Result {
    Playground::setup("ucp_test_29", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("source.txt", "fake data"),
            FileWithContent("source_with_body.txt", "some-body"),
        ]);
        let () = test().cwd(dirs.root()).run(format!(
            "cp --no-clobber ucp_test_29/{} ucp_test_29/{}",
            "source.txt", "dest.txt"
        ))?;
        assert!(dirs.test().join("dest.txt").exists());

        let () = test().cwd(dirs.root()).run(format!(
            "cp --no-clobber ucp_test_29/{} ucp_test_29/{}",
            "source_with_body.txt", "dest.txt"
        ))?;
        // Should have same contents of original empty file as --no-clobber should not overwrite dest.txt
        assert_eq!(
            fs::read_to_string(dirs.test().join("dest.txt"))?,
            "fake data"
        );
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_cp_debug_default() -> Result {
    Playground::setup("ucp_test_30", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual: CompleteResult = test().cwd(dirs.root()).run_with_data(
            RUNNER,
            format!(
                "cp --debug `{}` ucp_test_30/{TEST_HELLO_WORLD_DEST}",
                src.display()
            ),
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
        if !actual.stdout.contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported",
        ) {
            panic!("Failure: stdout was \n{}", actual.stdout);
        }

        #[cfg(windows)]
        if !actual.stdout.contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported",
        ) {
            panic!("Failure: stdout was \n{}", actual.stdout);
        }
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn test_cp_verbose_default() -> Result {
    Playground::setup("ucp_test_31", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual: CompleteResult = test().cwd(dirs.root()).run_with_data(
            RUNNER,
            format!("cp --verbose `{}` {TEST_HELLO_WORLD_DEST}", src.display()),
        )?;
        assert_contains(
            format!(
                "'{}' -> '{}'",
                src.display(),
                dirs.root().join(TEST_HELLO_WORLD_DEST).display()
            ),
            actual.stdout,
        );
        Ok(())
    })
}

#[test]
fn test_cp_only_source_no_dest() -> Result {
    Playground::setup("ucp_test_32", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let err = test()
            .cwd(dirs.root())
            .run(format!("cp {}", src.display(),))
            .expect_shell_error()?;
        let msg = err.generic_msg()?;
        assert_contains("Missing destination path operand after", &msg);
        assert_contains(TEST_HELLO_WORLD_SOURCE, &msg);
        Ok(())
    })
}

#[test]
fn test_cp_with_vars() -> Result {
    Playground::setup("ucp_test_33", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("input")]);
        let () = test()
            .cwd(dirs.test())
            .run("let src = 'input'; let dst = 'target'; cp $src $dst")?;
        assert!(dirs.test().join("target").exists());
        Ok(())
    })
}

#[test]
fn test_cp_destination_after_cd() -> Result {
    Playground::setup("ucp_test_34", |dirs, sandbox| {
        sandbox.mkdir("test");
        sandbox.with_files(&[EmptyFile("test/file.txt")]);
        let () = test().cwd(dirs.test()).run(
            // Defining variable avoid path expansion of cp argument.
            // If argument was not expanded ucp wrapper should do it
            "cd test; let file = 'copy.txt'; cp file.txt $file",
        )?;
        assert!(dirs.test().join("test").join("copy.txt").exists());
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg(not(windows))]
#[case("'a]?c'")]
#[cfg(not(windows))]
#[case("'a*.?c'")]
fn copies_files_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("ucp_test_34", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let () = test()
            .cwd(dirs.test())
            .run(format!("cp '{}' {TEST_HELLO_WORLD_DEST}", src.display(),))?;

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
#[cfg(not(windows))]
#[case("'a]?c'")]
#[cfg(not(windows))]
#[case("'a*.?c'")]
fn copies_files_with_glob_metachars_when_input_are_variables(#[case] src_name: &str) -> Result {
    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        let () = test().cwd(dirs.test()).run(format!(
            "let f = '{}'; cp $f {TEST_HELLO_WORLD_DEST}",
            src.display(),
        ))?;

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());
        Ok(())
    })
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_timestamps() -> Result {
    // Preserve timestamp and mode

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
        let code = "
            chmod +x file.txt
            cp --preserve [ mode timestamps ] file.txt other.txt

            let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
            let new_attrs = ls -l other.txt | get 0 | select mode accessed modified

            $old_attrs == $new_attrs
        ";

        test()
            .cwd(dirs.test())
            .inherit_path()
            .run(code)
            .expect_value_eq(true)
    })
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_only_timestamps() -> Result {
    // Preserve timestamps and discard all other attributes including mode

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
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
            .cwd(dirs.test())
            .inherit_path()
            .run(code)
            .expect_value_eq([true, true])
    })
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_nothing() -> Result {
    // Preserve no attributes

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
        let code = "
            chmod +x file.txt
            cp --preserve [] file.txt other.txt

            let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
            let new_attrs = ls -l other.txt | get 0 | select mode accessed modified

            $old_attrs != $new_attrs
        ";

        test()
            .cwd(dirs.test())
            .inherit_path()
            .run(code)
            .expect_value_eq(true)
    })
}

#[test]
fn test_cp_inside_glob_metachars_dir() -> Result {
    Playground::setup("open_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        let () = test()
            .cwd(dirs.test().join(sub_dir))
            .run("cp test_file.txt ../")?;

        assert!(files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        assert!(files_exist_at(&["test_file.txt"], dirs.test()));
        Ok(())
    })
}

#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn test_cp_to_customized_home_directory() -> Result {
    Playground::setup("cp_to_home", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);
        let code = "mkdir test; cp test_file.txt ~/test/";
        let result: CompleteResult = test()
            .cwd(dirs.test())
            .env("HOME", dirs.test())
            .run_with_data("nu -n -c $in | complete", code)?;

        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert!(files_exist_at(&["test_file.txt"], dirs.test().join("test")));
        Ok(())
    })
}

#[test]
fn cp_with_tilde() -> Result {
    Playground::setup("cp_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);
        sandbox.within("~tilde2");
        // cp directory
        test()
            .cwd(dirs.test())
            .run("let f = '~tilde'; cp -r $f '~tilde2'; ls '~tilde2/~tilde' | length")
            .expect_value_eq(3)?;

        // cp file
        let () = test().cwd(dirs.test()).run("cp '~tilde/f1.txt' ./")?;
        assert!(files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));

        // pass variable
        let () = test()
            .cwd(dirs.test())
            .run("let f = '~tilde/f2.txt'; cp $f ./")?;
        assert!(files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));
        Ok(())
    })
}

#[rstest]
#[case::without_progress("")]
#[case::with_progress("--progress")]
#[nu_test_support::test]
#[deps(NU)]
fn copy_file_with_update_flag(#[case] progress_flag: &str) -> Result {
    Playground::setup("cp_test_36", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        let code = format!("cp {progress_flag} -u valid.txt newer_valid.txt; open newer_valid.txt");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert_eq!(result.stdout.trim(), "body");

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let code = format!("cp {progress_flag} -u newest_valid.txt valid.txt; open valid.txt");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert_eq!(result.stdout.trim(), "newest_body");

        // when destination doesn't exist
        let code =
            format!("cp {progress_flag} -u newest_valid.txt des_missing.txt; open des_missing.txt");
        let result: CompleteResult = test().cwd(sandbox.cwd()).run_with_data(RUNNER, code)?;
        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert_eq!(result.stdout.trim(), "newest_body");
        Ok(())
    })
}

#[test]
fn cp_with_cd() -> Result {
    Playground::setup("cp_test_37", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(&[FileWithContent("tmp_dir/file.txt", "body")]);

        test()
            .cwd(sandbox.cwd())
            .run("do { cd tmp_dir; let f = 'file.txt'; cp $f .. }; open file.txt")
            .expect_value_eq("body")
    })
}

#[test]
fn test_cp_wildcards() -> Result {
    Playground::setup("cp_with_wildcards", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent(".a", "hello")]);

        let err = test()
            .cwd(dirs.test().join(sub_dir))
            .run("cp * ../")
            .expect_shell_error()?;
        // by default, wildcard don't match dot files.
        assert_contains("FileNotFound", format!("{err:?}"));
        assert!(files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(!files_exist_at(&[".a"], dirs.test()));

        // unless `-a` flag is provided.
        let () = test().cwd(dirs.test().join(sub_dir)).run("cp -a * ../")?;
        // by default, wildcard don't match dot files.
        assert!(files_exist_at(&[".a"], dirs.test().join(sub_dir)));
        assert!(files_exist_at(&[".a"], dirs.test()));
        Ok(())
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn cp_literal_directory_with_recursive_flag() -> Result {
    Playground::setup("cp_literal_dir_dc", |dirs, sandbox| {
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("test.txt")]);
        sandbox.mkdir("dest");

        let () = test()
            .cwd(dirs.root())
            .run("cp cp_literal_dir_dc/subdir cp_literal_dir_dc/dest --recursive")?;

        assert!(dirs.test().join("dest/subdir/test.txt").exists());
        Ok(())
    })
}
