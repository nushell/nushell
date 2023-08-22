
use filetime::FileTime;
use nu_test_support::fs::file_contents;
use nu_test_support::fs::{
    files_exist_at, AbsoluteFile,
    Stub::{EmptyFile, FileWithContent, FileWithPermission},
};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
#[cfg(all(unix, not(target_os = "freebsd")))]
use std::os::unix::fs::MetadataExt;

#[cfg(all(unix, not(target_os = "freebsd")))]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn get_file_hash<T: std::fmt::Display>(file: T) -> String {
    nu!("open -r {} | to text | hash md5", file).out
}

/// Assert that mode, ownership, and permissions of two metadata objects match.
#[cfg(all(not(windows), not(target_os = "freebsd")))]
macro_rules! assert_metadata_eq {
    ($m1:expr, $m2:expr) => {{
        assert_eq!($m1.mode(), $m2.mode(), "mode is different");
        assert_eq!($m1.uid(), $m2.uid(), "uid is different");
        assert_eq!($m1.atime(), $m2.atime(), "atime is different");
        assert_eq!(
            $m1.atime_nsec(),
            $m2.atime_nsec(),
            "atime_nsec is different"
        );
        assert_eq!($m1.mtime(), $m2.mtime(), "mtime is different");
        assert_eq!(
            $m1.mtime_nsec(),
            $m2.mtime_nsec(),
            "mtime_nsec is different"
        );
    }};
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

#[test]
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
        assert!(
            actual.err.contains("invalid_prem.txt") && actual.err.contains("Permission denied")
        );
    });
}

#[test]
fn copy_file_with_update_flag() {
    copy_file_with_update_flag_impl(false);
    copy_file_with_update_flag_impl(true);
}

fn copy_file_with_update_flag_impl(progress: bool) {
    Playground::setup("cp_test_19", |_dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            "cp {} --update=older valid.txt newer_valid.txt; open newer_valid.txt",
            progress_flag,
        );
        assert!(actual.out.contains("body"));

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(vec![FileWithContent("newest_valid.txt", "newest_body")]);
        let actual = nu!(cwd: sandbox.cwd(), "cp {} --update=older newest_valid.txt valid.txt; open valid.txt", progress_flag);
        assert_eq!(actual.out, "newest_body");

        // when destination doesn't exist
        let actual = nu!(cwd: sandbox.cwd(), "cp {} --update=older newest_valid.txt des_missing.txt; open des_missing.txt", progress_flag);
        assert_eq!(actual.out, "newest_body");
    });
}

// uutils/coreutils copy tests
static TEST_EXISTING_FILE: &str = "existing_file.txt";
static TEST_HELLO_WORLD_SOURCE: &str = "hello_world.txt";
static TEST_HELLO_WORLD_SOURCE_SYMLINK: &str = "hello_world.txt.link";
static TEST_HELLO_WORLD_DEST: &str = "copy_of_hello_world.txt";
static TEST_HELLO_WORLD_DEST_SYMLINK: &str = "copy_of_hello_world.txt.link";
static TEST_HOW_ARE_YOU_SOURCE: &str = "how_are_you.txt";
static TEST_HOW_ARE_YOU_DEST: &str = "hello_dir/how_are_you.txt";
static TEST_COPY_TO_FOLDER: &str = "hello_dir/";
static TEST_COPY_TO_FOLDER_FILE: &str = "hello_dir/hello_world.txt";
static TEST_COPY_FROM_FOLDER: &str = "hello_dir_with_file/";
static TEST_COPY_FROM_FOLDER_FILE: &str = "hello_dir_with_file/hello_world.txt";
static TEST_COPY_TO_FOLDER_NEW: &str = "hello_dir_new";
static TEST_COPY_TO_FOLDER_NEW_FILE: &str = "hello_dir_new/hello_world.txt";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_COPY_FROM_FOLDER: &str = "dir_with_mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_MOUNTPOINT: &str = "mount";
#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd"))]
static TEST_MOUNT_OTHER_FILESYSTEM_FILE: &str = "mount/DO_NOT_copy_me.txt";
#[cfg(unix)]
static TEST_NONEXISTENT_FILE: &str = "nonexistent_file.txt";

// #[test]
// fn test_cp_cp() {
//     let (at, mut ucmd) = at_and_ucmd!();
//     // Invoke our binary to make the copy.
//     ucmd.arg(TEST_HELLO_WORLD_SOURCE)
//         .arg(TEST_HELLO_WORLD_DEST)
//         .succeeds();

//     // Check the content of the destination file that was copied.
//     assert_eq!(at.read(TEST_HELLO_WORLD_DEST), "Hello, World!\n");
// }

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_cp() {
    Playground::setup("ucp_test_1", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        nu!(
            cwd: dirs.root(),
            "cp {} ucp_test_1/{}",
            src.display(),
            TEST_HELLO_WORLD_DEST
        );

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_existing_target() {
    Playground::setup("ucp_test_2", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let existing = dirs.fixtures.join("cp").join(TEST_EXISTING_FILE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        // Copy existing file to destination, so that it exists for the test
        nu!(
            cwd: dirs.root(),
            "cp {} ucp_test_2/{}",
            existing.display(),
            TEST_EXISTING_FILE
        );

        // At this point the src and existing files should be different
        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Now for the test
        nu!(
            cwd: dirs.root(),
            "cp {} ucp_test_2/{}",
            src.display(),
            TEST_EXISTING_FILE
        );

        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_EXISTING_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_multiple_files() {
    Playground::setup("ucp_test_3", |dirs, sandbox| {
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
            "cp {} {} ucp_test_3/{}",
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

#[cfg(feature = "nuuu")]
#[test]
#[cfg(not(target_os = "macos"))]
fn test_cp_recurse() {
    Playground::setup("ucp_test_4", |dirs, sandbox| {
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
            "cp -r {} ucp_test_4/{}",
            TEST_COPY_FROM_FOLDER,
            TEST_COPY_TO_FOLDER_NEW,
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_NEW_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_with_dirs_t() {
    Playground::setup("ucp_test_5", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        nu!(
            cwd: dirs.root(),
            "cp -t ucp_test_5/{} {}",
            TEST_COPY_TO_FOLDER,
            src.display()
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE).display());
        assert_eq!(src_hash, after_cp_hash);

        // Alternate orders on which arguments are given
        nu!(
            cwd: dirs.root(),
            "cp {} -t ucp_test_5/{}",
            src.display(),
            TEST_COPY_TO_FOLDER,
        );

        let after_cp_2_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE).display());
        assert_eq!(src_hash, after_cp_2_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_with_dirs() {
    Playground::setup("ucp_test_6", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        nu!(
            cwd: dirs.root(),
            "cp {} ucp_test_6/{}",
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
            "cp {} ucp_test_6/{}",
            src2.display(),
            TEST_HELLO_WORLD_DEST,
        );
        let after_cp_2_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src2_hash, after_cp_2_hash);
    });
}

#[cfg(feature = "nuuu")]
#[cfg(ignore)]
#[test]
fn test_cp_arg_no_target_directory() {
    // FIX THIS TEST TO CHECK THAT THIS GOES TO STDERR..NOT YET
    Playground::setup("ucp_test_7", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());
        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        let actual = nu!(
            cwd: dirs.root(),
            "cp {} -v -T ucp_test_7/{}",
            src.display(),
            TEST_COPY_TO_FOLDER,
        );

        // assert!(actual.err.contains("../formats"));
        // assert!(actual.err.contains("resolves to a directory (not copied)"));
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_arg_update_none() {
    // use the test commented above to create new one here
    Playground::setup("ucp_test_8", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let target = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let target_content = file_contents(&target);

        let target_hash = get_file_hash(src.display());
        // Start test
        let actual = nu!(
            cwd: dirs.root(),
            "cp {} {} --update=none",
            src.display(),
            target.display()
        );
        // File exists, so it  doesnt do anything
        // and original target didnt change
        assert_eq!(target_content, file_contents(target));
        // assert_eq!(target_hash, after_cp_hash);
    });
}
#[cfg(feature = "nuuu")]
#[test]
fn test_cp_arg_symlink() {
    Playground::setup("ucp_test_9", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        // Start test
        let actual = nu!(
            cwd: dirs.root(),
            "cp {} --symbolic-link ucp_test_9/{}",
            src.display(),
            TEST_HELLO_WORLD_DEST
        );
        let dst_after_cp = dirs.test().join(TEST_HELLO_WORLD_DEST);
        assert!(dst_after_cp.exists() && dst_after_cp.is_symlink());
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_arg_backup() {
    Playground::setup("ucp_test_10", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.with_files(vec![FileWithContent(
            "existing_file.txt",
            "hi_from_existing",
        )]);

        let actual = nu!(
            cwd: dirs.root(),
            "cp {} ucp_test_10/{} -b",
            src.display(),
            "existing_file.txt"
        );
        let backup_file = dirs.test().join("existing_file.txt~");
        assert!(&backup_file.exists());

        // Assert backup content is same as original
        assert_eq!(file_contents(backup_file), "hi_from_existing");

        //Assert a copy was made which copied the src contents
        assert_eq!(
            file_contents(src),
            file_contents(dirs.test().join("existing_file.txt"))
        );
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_arg_suffix() {
    Playground::setup("ucp_test_11", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.with_files(vec![FileWithContent(
            "existing_file.txt",
            "hi_from_existing",
        )]);

        let actual = nu!(
            cwd: dirs.root(),
            "cp {} -b --suffix .bak ucp_test_11/{}",
            src.display(),
            "existing_file.txt"
        );
        let backup_file = dirs.test().join("existing_file.txt.bak");
        assert!(&backup_file.exists());

        // Assert backup content is same as original
        assert_eq!(file_contents(backup_file), "hi_from_existing");

        //Assert a copy was made which copied the src contents
        assert_eq!(
            file_contents(src),
            file_contents(dirs.test().join("existing_file.txt"))
        );
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_custom_backup_suffix_via_env() {
    Playground::setup("ucp_test_12", |dirs, sandbox| {
        let suffix = "super-suffix-of-the-century";
        // sandbox.with_env("SIMPLE_BACKUP_SUFFIX", suffix);
        // add environment variable to the test
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.with_files(vec![FileWithContent(
            "existing_file.txt",
            "hi_from_existing",
        )]);
        // This fails because i cant figure out how to add the ENV variable
        // not sandbox.with_env()

        // not this either
        // let actual = nu!("$env.SIMPLE_BACKUP_SUFFIX = 'foo'");
        // but if i try it normally, it seems to work just fine??
        // help :(
        let actual = nu!(
            cwd: dirs.root(),
            "$env.SIMPLE_BACKUP_SUFFIX = {}; cp -b {} ucp_test_12/{}",
            suffix,
            src.display(),
            "existing_file.txt"
        );
        let backup_file = dirs.test().join(format!("existing_file.txt{}", suffix));

        assert!(&backup_file.exists());

        // Assert backup content is same as original
        assert_eq!(file_contents(backup_file), "hi_from_existing");

        //Assert a copy was made which copied the src contents
        assert_eq!(
            file_contents(src),
            file_contents(dirs.test().join("existing_file.txt"))
        );
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_deref() {
    Playground::setup("ucp_test_13", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.symlink(
            src.display().to_string(),
            dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
        );
        let symlink_path = dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK);

        //Make directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // sandbox.with_env("SIMPLE_BACKUP_SUFFIX", suffix);
        // add environment variable to the test
        let actual = nu!(
        cwd: dirs.root(),
        "cp -L {} {} ucp_test_13/{}",
        src.display(),
        symlink_path.display(),
        TEST_COPY_TO_FOLDER
        );
        let after_cp_file1 = dirs
            .test()
            .join(TEST_COPY_TO_FOLDER)
            .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
        assert!(after_cp_file1.exists());
        // unlike -P/--no-deref, we expect a file, not a link
        assert!(!after_cp_file1.is_symlink());

        let after_cp_file2 = dirs
            .test()
            .join(TEST_COPY_TO_FOLDER)
            .join(TEST_HELLO_WORLD_SOURCE);

        // Check the content of the destination file that was copied.
        assert_eq!(file_contents(after_cp_file1), file_contents(&src));
        assert_eq!(file_contents(after_cp_file2), file_contents(&src));
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_no_deref() {
    Playground::setup("ucp_test_14", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.symlink(
            src.display().to_string(),
            dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
        );
        let symlink_path = dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK);

        //Make directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // sandbox.with_env("SIMPLE_BACKUP_SUFFIX", suffix);
        // add environment variable to the test
        let actual = nu!(
        cwd: dirs.root(),
        "cp -P {} {} ucp_test_14/{}",
        src.display(),
        symlink_path.display(),
        TEST_COPY_TO_FOLDER
        );
        let after_cp_symlink = dirs
            .test()
            .join(TEST_COPY_TO_FOLDER)
            .join(TEST_HELLO_WORLD_SOURCE_SYMLINK);
        assert!(after_cp_symlink.exists());
        // We expect a symlink now
        assert!(after_cp_symlink.is_symlink());

        let after_cp_file = dirs
            .test()
            .join(TEST_COPY_TO_FOLDER)
            .join(TEST_HELLO_WORLD_SOURCE);

        // Check the content of the destination file that was copied.
        assert_eq!(file_contents(after_cp_symlink), file_contents(&src));
        assert_eq!(file_contents(after_cp_file), file_contents(&src));
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_no_deref_link_onto_link() {
    Playground::setup("ucp_test_15", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        sandbox.symlink(
            src.display().to_string(),
            dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
        );
        let symlink_path = dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK);

        // sandbox.with_env("SIMPLE_BACKUP_SUFFIX", suffix);
        // add environment variable to the test
        let actual = nu!(
        cwd: dirs.root(),
        "cp -P {} ucp_test_15/{}",
        symlink_path.display(),
        TEST_HELLO_WORLD_DEST_SYMLINK
        );
        let after_cp_symlink = dirs.test().join(TEST_HELLO_WORLD_DEST_SYMLINK);
        // Check symlink copied onto another symlink with same src
        assert!(after_cp_symlink.is_symlink());
        // Check the content of the destination file that was copied.
        assert_eq!(file_contents(after_cp_symlink), file_contents(&src));
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_strip_trailing_slashes() {
    Playground::setup("ucp_test_16", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());

        let actual = nu!(
        cwd: dirs.root(),
        "cp --strip-trailing-slashes {} ucp_test_16/{} ",
        format!("{}/", src.display()),
        TEST_HELLO_WORLD_DEST
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        // Check the content of the destination file that was copied.
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_parents() {
    Playground::setup("ucp_test_17", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src_hash = get_file_hash(src.display());

        // Make directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        nu!(
        cwd: dirs.root(),
        "cp --parents {} ucp_test_17/{}",
        src.display(),
        // TEST_COPY_FROM_FOLDER_FILE, // hello_dir_with_file/hello_world.txt
        TEST_COPY_TO_FOLDER // hello_dir
        );
        let after_cp_hash =
            get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER).join(src).display());
        // Check the content of the destination file that was copied.
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_parents_multiple_files() {
    Playground::setup("ucp_test_18", |dirs, sandbox| {
        let src1 = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src1_hash = get_file_hash(src1.display());
        let src2 = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let src2_hash = get_file_hash(src2.display());

        // Make directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);

        nu!(
        cwd: dirs.root(),
        "cp --parents {} {} ucp_test_18/{}",
        src1.display(),
        src2.display(),
        TEST_COPY_TO_FOLDER // hello_dir
        );
        let after_cp_hash1 =
            get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER).join(src1).display());
        let after_cp_hash2 =
            get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER).join(src2).display());
        assert_eq!(src1_hash, after_cp_hash1);
        assert_eq!(src2_hash, after_cp_hash2);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_parents_dest_not_directory() {
    Playground::setup("ucp_test_19", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src_hash = get_file_hash(src.display());

        let actual = nu!(
        cwd: dirs.root(),
        "cp --parents {} ucp_test_19/{}",
        src.display(),
        TEST_HELLO_WORLD_DEST
        );
        actual
            .err
            .contains("with --parents, the destination must be a directory");
    });
}

#[cfg(feature = "nuuu")]
#[cfg(not(windows))]
#[test]
fn test_cp_arg_force() {
    Playground::setup("ucp_test_20", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());
        sandbox.with_files(vec![FileWithPermission("invalid_prem.txt", false)]);

        let actual = nu!(
        cwd: dirs.root(),
        "cp {} --force ucp_test_20/{}",
        src.display(),
        "invalid_prem.txt"
        );
        let after_cp_hash = get_file_hash(dirs.test().join("invalid_prem.txt").display());
        // Check content was copied by the use of --force
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_arg_remove_destination() {
    Playground::setup("ucp_test_21", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());
        sandbox.with_files(vec![FileWithPermission("invalid_prem.txt", false)]);
        let actual = nu!(
        cwd: dirs.root(),
        "cp {} --remove-destination ucp_test_21/{}",
        src.display(),
        "invalid_prem.txt"
        );
        let after_cp_hash = get_file_hash(dirs.test().join("invalid_prem.txt").display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_preserve_timestamps() {
    Playground::setup("ucp_test_22", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("a.txt"), EmptyFile("b.txt")]);
        // let target = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let one_h = SystemTime::now() - Duration::from_secs(3600);

        let previous = FileTime::from_system_time(one_h);
        let file = dirs.test().join("a.txt");
        filetime::set_file_times(file, previous, previous).unwrap();
        let actual = nu!(
        cwd: dirs.root(),
        "cp {} --preserve [timestamps] ucp_test_22/{}",
        dirs.test().join("a.txt").display(),
        TEST_HOW_ARE_YOU_SOURCE
        );
        let metadata = std::fs::metadata(dirs.test().join("a.txt")).unwrap();
        let creation = metadata.modified().unwrap();

        // let after_cp_hash = get_file_hash(dirs.test().join(TEST_HOW_ARE_YOU_SOURCE).display());

        // assert!(dirs.test().join(TEST_HOW_ARE_YOU_SOURCE).exists());
        let metadata2 = std::fs::metadata(dirs.test().join(TEST_HOW_ARE_YOU_SOURCE)).unwrap();
        let creation2 = metadata2.modified().unwrap();
        assert_eq!(creation, creation2);
    });
}

#[cfg(feature = "nuuu")]
#[cfg(target_os = "linux")]
#[test]
fn test_cp_arg_link() {
    Playground::setup("ucp_test_23", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("hi.txt")]);
        nu!(
        cwd: dirs.root(),
        "cp {} --link ucp_test_23/{}",
        dirs.test().join("hi.txt").display(),
        TEST_HELLO_WORLD_DEST
        );
        let metadata = std::fs::metadata(dirs.test().join("hi.txt")).unwrap();
        assert_eq!(metadata.nlink(), 2);
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_copy_symlink_contents_recursive() {
    Playground::setup("ucp_test_24", |dirs, sandbox| {
        sandbox.mkdir("src-dir");
        sandbox.mkdir("dest-dir");
        assert!(dirs.test().join("src-dir").exists());
        // assert_eq!(dirs.root(), dirs.test());
        sandbox.with_files(vec![FileWithContent("f", "f")]);

        // sandbox.symlink(
        //     src.display().to_string(),
        //     dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK),
        // );
        // let symlink_path = dirs.test().join(TEST_HELLO_WORLD_SOURCE_SYMLINK);

        sandbox.symlink(dirs.test().join("f"), dirs.test().join("slink"));
        sandbox.symlink("no-file", dirs.test().join("src-dir").join("slink"));
        // sandbox.with_symlink_file("f", "slink");
        // sandbox.with_symlink_file("no-file", &path_concat!("src-dir", "slink"));
        nu!(
        cwd: dirs.root(),
        "cp -H -r {} ucp_test_24/{} ucp_test_24/{}",
        dirs.test().join("slink").display(),
        "src-dir",
        "dest-dir"

        // dirs.test().join("src-dir").display(),
        // dirs.test().join("dest-dir").display(),
        );
        assert!(dirs.test().join("src-dir").exists());
        assert!(dirs.test().join("dest-dir").exists());
        assert!(dirs.test().join("dest-dir").join("src-dir").exists());
        let regular_file = dirs.test().join("dest-dir").join("slink");
        assert!(regular_file.exists());
        assert!(!regular_file.is_symlink());
        assert_eq!(file_contents(&regular_file), "f");
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_directory_to_itself_disallowed() {
    Playground::setup("ucp_test_25", |dirs, sandbox| {
        sandbox.mkdir("d");
        let actual = nu!(
        cwd: dirs.root(),
        "cp -r ucp_test_25/{}  ucp_test_25/{}",
        "d",
        "d"
        );
        actual
            .err
            .contains("cannot copy a directory, 'd', into itself, 'd/d'");
    });
}

#[cfg(feature = "nuuu")]
#[test]
fn test_cp_nested_directory_to_itself_disallowed() {
    Playground::setup("ucp_test_25", |dirs, sandbox| {
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

#[cfg(feature = "nuuu")]
#[cfg(all(not(windows), not(target_os = "freebsd")))]
#[test]
fn test_cp_dir_preserve_permissions() {
    Playground::setup("ucp_test_26", |dirs, sandbox| {
        sandbox.mkdir("d1");
        let mut perms = std::fs::metadata(dirs.test().join("d1"))
            .unwrap()
            .permissions();
        perms.set_mode(0o0500);
        std::fs::set_permissions(dirs.test().join("d1"), perms).unwrap();
        nu!(
        cwd: dirs.test(),
        "cp -p -r {} {}",
        "d1",
        "d2"
        );
        assert!(dirs.test().join("d2").exists());
        let metadata1 = std::fs::metadata(dirs.test().join("d1")).unwrap();
        let after_cp_metadata = std::fs::metadata(dirs.test().join("d2")).unwrap();
        assert_metadata_eq!(metadata1, after_cp_metadata);
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_same_file_force() {
    Playground::setup("ucp_test_27", |dirs, sandbox| {
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

// WRITE TEST FOR GLOBBING STUFF
// GLOBBING IS NOT WORKING ON SOURCE FIX THAT ON UCP
// LOOK AT CP.RS FOR INSPIRATIOn
