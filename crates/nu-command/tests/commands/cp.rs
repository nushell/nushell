use nu_test_support::fs::file_contents;
use nu_test_support::fs::{
    files_exist_at, AbsoluteFile,
    Stub::{EmptyFile, FileWithContent, FileWithPermission},
};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

use std::path::Path;

fn get_file_hash<T: std::fmt::Display>(file: T) -> String {
    nu!("open -r {} | to text | hash md5", file).out
}

#[test]
fn copies_a_file() {
    copies_a_file_impl(false);
    copies_a_file_impl(true);
}

fn copies_a_file_impl(progress: bool) {
    Playground::setup("cp_test_1", |dirs, _| {
        let test_file = dirs.formats().join("sample.ini");
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let first_hash = get_file_hash(test_file.display());

        nu!(
            cwd: dirs.root(),
            "cp {} `{}` cp_test_1/sample.ini",
            progress_flag,
            test_file.display()
        );

        assert!(dirs.test().join("sample.ini").exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join("sample.ini").display());
        assert_eq!(first_hash, after_cp_hash);
    });
}

#[test]
fn copies_the_file_inside_directory_if_path_to_copy_is_directory() {
    copies_the_file_inside_directory_if_path_to_copy_is_directory_impl(false);
    copies_the_file_inside_directory_if_path_to_copy_is_directory_impl(true);
}

fn copies_the_file_inside_directory_if_path_to_copy_is_directory_impl(progress: bool) {
    Playground::setup("cp_test_2", |dirs, _| {
        let expected_file = AbsoluteFile::new(dirs.test().join("sample.ini"));
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let first_hash = get_file_hash(dirs.formats().join("../formats/sample.ini").display());
        nu!(
            cwd: dirs.formats(),
            "cp {} ../formats/sample.ini {}",
            progress_flag,
            expected_file.dir()
        );

        assert!(dirs.test().join("sample.ini").exists());

        // Check the integrity of the file.
        let after_cp_hash = get_file_hash(expected_file);
        assert_eq!(first_hash, after_cp_hash);
    })
}

// error msg changes on coreutils
#[test]
fn error_if_attempting_to_copy_a_directory_to_another_directory() {
    error_if_attempting_to_copy_a_directory_to_another_directory_impl(false);
    error_if_attempting_to_copy_a_directory_to_another_directory_impl(true);
}

fn error_if_attempting_to_copy_a_directory_to_another_directory_impl(progress: bool) {
    Playground::setup("cp_test_3", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };
        let actual = nu!(
            cwd: dirs.formats(),
            "cp {} ../formats {}",
            progress_flag,
            dirs.test().display()
        );

        // Changing to GNU error like error
        // Slight bug since it should say formats, but its saying "." due to the `strip_prefix`
        // that i do I think
        // assert!(actual.err.contains("formats"));
        // assert!(actual.err.contains("resolves to a directory (not copied)"));
        assert!(actual.err.contains("omitting directory"));

        // directories must be copied using --recursive
        // gnu says "omitting directory", vbecause -r was not given
    });
}

#[test]
fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag() {
    copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag_impl(
        false,
    );
    copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag_impl(
        true,
    );
}

fn copies_the_directory_inside_directory_if_path_to_copy_is_directory_and_with_recursive_flag_impl(
    progress: bool,
) {
    Playground::setup("cp_test_4", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");
        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test(),
            "cp {} originals expected -r",
            progress_flag
        );

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            vec![
                Path::new("yehuda.txt"),
                Path::new("jttxt"),
                Path::new("andres.txt")
            ],
            &expected_dir
        ));
    })
}

#[test]
fn deep_copies_with_recursive_flag() {
    deep_copies_with_recursive_flag_impl(false);
    deep_copies_with_recursive_flag_impl(true);
}

fn deep_copies_with_recursive_flag_impl(progress: bool) {
    Playground::setup("cp_test_5", |dirs, sandbox| {
        sandbox
            .within("originals")
            .with_files(vec![EmptyFile("manifest.txt")])
            .within("originals/contributors")
            .with_files(vec![
                EmptyFile("yehuda.txt"),
                EmptyFile("jttxt"),
                EmptyFile("andres.txt"),
            ])
            .within("originals/contributors/JT")
            .with_files(vec![EmptyFile("errors.txt"), EmptyFile("multishells.txt")])
            .within("originals/contributors/andres")
            .with_files(vec![EmptyFile("coverage.txt"), EmptyFile("commands.txt")])
            .within("originals/contributors/yehuda")
            .with_files(vec![EmptyFile("defer-evaluation.txt")])
            .mkdir("expected");

        let expected_dir = dirs.test().join("expected").join("originals");
        let progress_flag = if progress { "-p" } else { "" };

        let jts_expected_copied_dir = expected_dir.join("contributors").join("JT");
        let andres_expected_copied_dir = expected_dir.join("contributors").join("andres");
        let yehudas_expected_copied_dir = expected_dir.join("contributors").join("yehuda");

        nu!(
            cwd: dirs.test(),
            "cp {} originals expected --recursive",
            progress_flag
        );

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            vec![Path::new("errors.txt"), Path::new("multishells.txt")],
            jts_expected_copied_dir
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
    copies_using_path_with_wildcard_impl(false);
    copies_using_path_with_wildcard_impl(true);
}

fn copies_using_path_with_wildcard_impl(progress: bool) {
    Playground::setup("cp_test_6", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let src_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls ../formats/*) { open --raw $file.name | to text | hash md5 }"
        )
        .out;

        nu!(
            cwd: dirs.formats(),
            "cp {} -r ../formats/* {}",
            progress_flag,
            dirs.test().display()
        );

        assert!(files_exist_at(
            vec![
                Path::new("caco3_plastics.csv"),
                Path::new("cargo_sample.toml"),
                Path::new("jt.xml"),
                Path::new("sample.ini"),
                Path::new("sgml_description.json"),
                Path::new("utf16.ini"),
            ],
            dirs.test()
        ));

        // Check integrity after the copy is done
        let dst_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls {}) {{ open --raw $file.name | to text | hash md5 }}", dirs.test().display()
        ).out;
        assert_eq!(src_hashes, dst_hashes);
    })
}

#[test]
fn copies_using_a_glob() {
    copies_using_a_glob_impl(false);
    copies_using_a_glob_impl(true);
}

fn copies_using_a_glob_impl(progress: bool) {
    Playground::setup("cp_test_7", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let src_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls *) { open --raw $file.name | to text | hash md5 }"
        )
        .out;

        nu!(
            cwd: dirs.formats(),
            "cp {} -r * {}",
            progress_flag,
            dirs.test().display()
        );

        assert!(files_exist_at(
            vec![
                Path::new("caco3_plastics.csv"),
                Path::new("cargo_sample.toml"),
                Path::new("jt.xml"),
                Path::new("sample.ini"),
                Path::new("sgml_description.json"),
                Path::new("utf16.ini"),
            ],
            dirs.test()
        ));

        // Check integrity after the copy is done
        let dst_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls {}) {{ open --raw $file.name | to text | hash md5 }}",
            dirs.test().display()
        )
        .out;
        assert_eq!(src_hashes, dst_hashes);
    });
}

#[test]
fn copies_same_file_twice() {
    copies_same_file_twice_impl(false);
    copies_same_file_twice_impl(true);
}

fn copies_same_file_twice_impl(progress: bool) {
    Playground::setup("cp_test_8", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.root(),
            "cp {} `{}` cp_test_8/sample.ini",
            progress_flag,
            dirs.formats().join("sample.ini").display()
        );

        nu!(
            cwd: dirs.root(),
            "cp {} `{}` cp_test_8/sample.ini",
            progress_flag,
            dirs.formats().join("sample.ini").display()
        );

        assert!(dirs.test().join("sample.ini").exists());
    });
}

#[test]
#[ignore = "Behavior not supported by uutils cp"]
fn copy_files_using_glob_two_parents_up_using_multiple_dots() {
    copy_files_using_glob_two_parents_up_using_multiple_dots_imp(false);
    copy_files_using_glob_two_parents_up_using_multiple_dots_imp(true);
}

fn copy_files_using_glob_two_parents_up_using_multiple_dots_imp(progress: bool) {
    Playground::setup("cp_test_9", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(vec![
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test().join("foo/bar"),
            " cp {} * ...",
            progress_flag,
        );

        assert!(files_exist_at(
            vec![
                "yehuda.yaml",
                "jtjson",
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
    copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive_impl(false);
    copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive_impl(true);
}

fn copy_file_and_dir_from_two_parents_up_using_multiple_dots_to_current_dir_recursive_impl(
    progress: bool,
) {
    Playground::setup("cp_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("hello_there")]);
        sandbox.mkdir("hello_again");
        sandbox.within("foo").mkdir("bar");

        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test().join("foo/bar"),
            "cp {} -r .../hello* .",
            progress_flag
        );

        let expected = dirs.test().join("foo/bar");

        assert!(files_exist_at(vec!["hello_there", "hello_again"], expected));
    })
}

// error msg changes on coreutils
#[test]
fn copy_to_non_existing_dir() {
    copy_to_non_existing_dir_impl(false);
    copy_to_non_existing_dir_impl(true);
}

fn copy_to_non_existing_dir_impl(progress: bool) {
    Playground::setup("cp_test_11", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("empty_file")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp {} empty_file ~/not_a_dir/",
            progress_flag
        );
        // assert!(actual.err.contains("failed to access"));
        assert!(actual.err.contains("is not a directory"));
    });
}

#[test]
fn copy_dir_contains_symlink_ignored() {
    copy_dir_contains_symlink_ignored_impl(false);
    copy_dir_contains_symlink_ignored_impl(true);
}

fn copy_dir_contains_symlink_ignored_impl(progress: bool) {
    Playground::setup("cp_test_12", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm {} tmp_dir/good_bye; cp -r tmp_dir tmp_dir_2",
            progress_flag
        );

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` don't exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(vec!["hello_there"], expected));
        // GNU cp will copy the broken symlink, so following their behavior
        // thus commenting out below
        // let path = expected.join("dangle_symlink");
        // assert!(!path.exists() && !path.is_symlink());
    });
}

#[test]
fn copy_dir_contains_symlink() {
    copy_dir_contains_symlink_impl(false);
    copy_dir_contains_symlink_impl(true);
}

fn copy_dir_contains_symlink_impl(progress: bool) {
    Playground::setup("cp_test_13", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm tmp_dir/good_bye; cp {} -r -n tmp_dir tmp_dir_2",
            progress_flag
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
    copy_dir_symlink_file_body_not_changed_impl(false);
    copy_dir_symlink_file_body_not_changed_impl(true);
}

fn copy_dir_symlink_file_body_not_changed_impl(progress: bool) {
    Playground::setup("cp_test_14", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(vec![EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            "rm tmp_dir/good_bye; cp {} -r -n tmp_dir tmp_dir_2; rm -r tmp_dir; cp {} -r -n tmp_dir_2 tmp_dir; echo hello_data | save tmp_dir/good_bye",
            progress_flag,
            progress_flag,
        );

        // check dangle_symlink in tmp_dir is no longer dangling.
        let expected_file = sandbox.cwd().join("tmp_dir").join("dangle_symlink");
        let actual = file_contents(expected_file);
        assert!(actual.contains("hello_data"));
    });
}

// error msg changes on coreutils
#[test]
fn copy_identical_file() {
    copy_identical_file_impl(false);
    copy_identical_file_impl(true);
}

fn copy_identical_file_impl(progress: bool) {
    Playground::setup("cp_test_15", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("same.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp {} same.txt same.txt",
            progress_flag,
        );
        // assert!(actual.err.contains("Copy aborted"));
        assert!(actual
            .err
            .contains("'same.txt' and 'same.txt' are the same file"));
    });
}

#[test]
#[ignore = "File name in progress bar not on uutils impl"]
fn copy_ignores_ansi() {
    copy_ignores_ansi_impl(false);
    copy_ignores_ansi_impl(true);
}

fn copy_ignores_ansi_impl(progress: bool) {
    Playground::setup("cp_test_16", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("test.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "ls | find test | get name | cp {} $in.0 success.txt; ls | find success | get name | ansi strip | get 0",
            progress_flag,
        );
        assert_eq!(actual.out, "success.txt");
    });
}

//apparently on windows error msg is different, but linux(where i test) is fine.
//fix later
#[cfg(unix)]
#[test]
fn copy_file_not_exists_dst() {
    copy_file_not_exists_dst_impl(false);
    copy_file_not_exists_dst_impl(true);
}

fn copy_file_not_exists_dst_impl(progress: bool) {
    Playground::setup("cp_test_17", |_dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("valid.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp {} valid.txt ~/invalid_dir/invalid_dir1",
            progress_flag,
        );
        assert!(
            actual.err.contains("invalid_dir1") && actual.err.contains("No such file or directory")
        );
    });
}

//again slightly different error message on windows on tests
// compared to linux
#[test]
#[ignore] //FIXME: This test needs to be re-enabled once uu_cp has fixed the bug
fn copy_file_with_read_permission() {
    copy_file_with_read_permission_impl(false);
    copy_file_with_read_permission_impl(true);
}

fn copy_file_with_read_permission_impl(progress: bool) {
    Playground::setup("cp_test_18", |_dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("valid.txt"),
            FileWithPermission("invalid_prem.txt", false),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp {} valid.txt invalid_prem.txt",
            progress_flag,
        );
        assert!(actual.err.contains("invalid_prem.txt") && actual.err.contains("denied"));
    });
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
fn test_cp_cp() {
    Playground::setup("cp_test_19", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_19/{}",
            src.display(),
            TEST_HELLO_WORLD_DEST
        );

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_existing_target() {
    Playground::setup("cp_test_20", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let existing = dirs.fixtures.join("cp").join(TEST_EXISTING_FILE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        // Copy existing file to destination, so that it exists for the test
        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_20/{}",
            existing.display(),
            TEST_EXISTING_FILE
        );

        // At this point the src and existing files should be different
        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Now for the test
        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_20/{}",
            src.display(),
            TEST_EXISTING_FILE
        );

        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_EXISTING_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_multiple_files() {
    Playground::setup("cp_test_21", |dirs, sandbox| {
        let src1 = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src2 = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src1_hash = get_file_hash(src1.display());
        let src2_hash = get_file_hash(src2.display());

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);

        // Start test
        nu!(
            cwd: dirs.root(),
            "cp {} {} cp_test_21/{}",
            src1.display(),
            src2.display(),
            TEST_COPY_TO_FOLDER
        );

        assert!(dirs.test().join(TEST_COPY_TO_FOLDER).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_1_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE).display());
        let after_cp_2_hash = get_file_hash(dirs.test().join(TEST_HOW_ARE_YOU_DEST).display());
        assert_eq!(src1_hash, after_cp_1_hash);
        assert_eq!(src2_hash, after_cp_2_hash);
    });
}

#[test]
#[cfg(not(target_os = "macos"))]
fn test_cp_recurse() {
    Playground::setup("cp_test_22", |dirs, sandbox| {
        // Create the relevant target directories
        sandbox.mkdir(TEST_COPY_FROM_FOLDER);
        sandbox.mkdir(TEST_COPY_TO_FOLDER_NEW);
        let src = dirs
            .fixtures
            .join("cp")
            .join(TEST_COPY_FROM_FOLDER)
            .join(TEST_COPY_FROM_FOLDER_FILE);

        let src_hash = get_file_hash(src.display());
        // Start test
        nu!(
            cwd: dirs.root(),
            "cp -r {} cp_test_22/{}",
            TEST_COPY_FROM_FOLDER,
            TEST_COPY_TO_FOLDER_NEW,
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_NEW_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_with_dirs() {
    Playground::setup("cp_test_23", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_23/{}",
            src.display(),
            TEST_COPY_TO_FOLDER,
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE).display());
        assert_eq!(src_hash, after_cp_hash);

        // Other way around
        sandbox.mkdir(TEST_COPY_FROM_FOLDER);
        let src2 = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src2_hash = get_file_hash(src2.display());
        nu!(
            cwd: dirs.root(),
            "cp {} cp_test_23/{}",
            src2.display(),
            TEST_HELLO_WORLD_DEST,
        );
        let after_cp_2_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src2_hash, after_cp_2_hash);
    });
}
#[cfg(not(windows))]
#[test]
fn test_cp_arg_force() {
    Playground::setup("cp_test_24", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());
        sandbox.with_files(vec![FileWithPermission("invalid_prem.txt", false)]);

        nu!(
        cwd: dirs.root(),
        "cp {} --force cp_test_24/{}",
        src.display(),
        "invalid_prem.txt"
        );
        let after_cp_hash = get_file_hash(dirs.test().join("invalid_prem.txt").display());
        // Check content was copied by the use of --force
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_directory_to_itself_disallowed() {
    Playground::setup("cp_test_25", |dirs, sandbox| {
        sandbox.mkdir("d");
        let actual = nu!(
        cwd: dirs.root(),
        "cp -r cp_test_25/{}  cp_test_25/{}",
        "d",
        "d"
        );
        actual
            .err
            .contains("cannot copy a directory, 'd', into itself, 'd/d'");
    });
}

#[test]
fn test_cp_nested_directory_to_itself_disallowed() {
    Playground::setup("cp_test_26", |dirs, sandbox| {
        sandbox.mkdir("a");
        sandbox.mkdir("a/b");
        sandbox.mkdir("a/b/c");
        let actual = nu!(
        cwd: dirs.test(),
        "cp -r {} {}",
        "a/b",
        "a/b/c"
        );
        actual
            .err
            .contains("cannot copy a directory, 'a/b', into itself, 'a/b/c/b'");
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_same_file_force() {
    Playground::setup("cp_test_27", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("f")]);
        let actual = nu!(
        cwd: dirs.test(),
        "cp --force {} {}",
        "f",
        "f"
        );
        actual.err.contains("cp: 'f' and 'f' are the same file");
        assert!(!dirs.test().join("f~").exists());
    });
}

#[test]
fn test_cp_arg_no_clobber() {
    Playground::setup("cp_test_28", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let target = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let target_hash = get_file_hash(target.display());

        let actual = nu!(
        cwd: dirs.root(),
        "cp {} {} --no-clobber",
        src.display(),
        target.display()
        );
        let after_cp_hash = get_file_hash(target.display());
        assert!(actual.err.contains("not replacing"));
        // Check content was not clobbered
        assert_eq!(after_cp_hash, target_hash);
    });
}

#[test]
fn test_cp_arg_no_clobber_twice() {
    Playground::setup("cp_test_29", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("source.txt"),
            FileWithContent("source_with_body.txt", "some-body"),
        ]);
        nu!(
        cwd: dirs.root(),
        "cp --no-clobber cp_test_29/{} cp_test_29/{}",
        "source.txt",
        "dest.txt"
        );
        assert!(dirs.test().join("dest.txt").exists());

        nu!(
        cwd: dirs.root(),
        "cp --no-clobber cp_test_29/{} cp_test_29/{}",
        "source_with_body.txt",
        "dest.txt"
        );
        // Should have same contents of original empty file as --no-clobber should not overwrite dest.txt
        assert_eq!(file_contents(dirs.test().join("dest.txt")), "fake data");
    });
}

#[test]
fn test_cp_debug_default() {
    Playground::setup("cp_test_30", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual = nu!(
        cwd: dirs.root(),
        "cp --debug {} cp_test_30/{}",
        src.display(),
        TEST_HELLO_WORLD_DEST
        );
        #[cfg(target_os = "macos")]
        if !actual
            .out
            .contains("copy offload: unknown, reflink: unsupported, sparse detection: unsupported")
        {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }
        #[cfg(target_os = "linux")]
        if !actual
            .out
            .contains("copy offload: unknown, reflink: unsupported, sparse detection: no")
        {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }

        #[cfg(windows)]
        if !actual.out.contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported",
        ) {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }
    });
}

#[test]
fn test_cp_verbose_default() {
    Playground::setup("cp_test_31", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual = nu!(
        cwd: dirs.root(),
        "cp --verbose {} cp_test_31/{}",
        src.display(),
        TEST_HELLO_WORLD_DEST
        );
        assert!(actual.out.contains(
            format!(
                "'{}' -> 'cp_test_31/{}'",
                src.display(),
                TEST_HELLO_WORLD_DEST
            )
            .as_str(),
        ));
    });
}

#[test]
fn test_cp_only_source_no_dest() {
    Playground::setup("cp_test_32", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let actual = nu!(
        cwd: dirs.root(),
        "cp {}",
        src.display(),
        );
        assert!(actual
            .err
            .contains("Missing destination path operand after"));
        assert!(actual.err.contains(TEST_HELLO_WORLD_SOURCE));
    });
}
