use nu_test_support::fs::file_contents;
use nu_test_support::fs::{
    Stub::{EmptyFile, FileWithContent, FileWithPermission},
    files_exist_at,
};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

use rstest::rstest;

#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: &str = "/";
#[cfg(target_os = "windows")]
const PATH_SEPARATOR: &str = "\\";

fn get_file_hash<T: std::fmt::Display>(file: T) -> String {
    nu!(format!("open -r {file} | to text | hash md5")).out
}

#[test]
fn copies_a_file() {
    copies_a_file_impl(false);
    copies_a_file_impl(true);
}

fn copies_a_file_impl(progress: bool) {
    Playground::setup("ucp_test_1", |dirs, _| {
        let test_file = dirs.formats().join("sample.ini");
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let first_hash = get_file_hash(test_file.display());

        nu!(
            cwd: dirs.root(),
            format!(
                "cp {progress_flag} `{}` ucp_test_1/sample.ini",
                test_file.display()
            )
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
    Playground::setup("ucp_test_2", |dirs, _| {
        let expected_file = dirs.test().join("sample.ini");
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let first_hash = get_file_hash(dirs.formats().join("../formats/sample.ini").display());
        nu!(
            cwd: dirs.formats(),
            format!(
                "cp {progress_flag} ../formats/sample.ini {}",
                expected_file.parent().unwrap().as_os_str().to_str().unwrap(),
            )
        );

        assert!(dirs.test().join("sample.ini").exists());

        // Check the integrity of the file.
        let after_cp_hash = get_file_hash(expected_file.display());
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
    Playground::setup("ucp_test_3", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };
        let actual = nu!(
            cwd: dirs.formats(),
            format!(
                "cp {progress_flag} ../formats {}",
                dirs.test().display()
            )
        );

        assert!(actual.err.contains("formats"));
        assert!(actual.err.contains("resolves to a directory (not copied)"));
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
        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test(),
            format!("cp {progress_flag} originals expected -r")
        );

        assert!(expected_dir.exists());
        assert!(files_exist_at(
            &["yehuda.txt", "jttxt", "andres.txt"],
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
        let progress_flag = if progress { "-p" } else { "" };

        let jts_expected_copied_dir = expected_dir.join("contributors").join("JT");
        let andres_expected_copied_dir = expected_dir.join("contributors").join("andres");
        let yehudas_expected_copied_dir = expected_dir.join("contributors").join("yehuda");

        nu!(
            cwd: dirs.test(),
            format!("cp {progress_flag} originals expected --recursive"),
        );

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
    })
}

#[test]
fn copies_using_path_with_wildcard() {
    copies_using_path_with_wildcard_impl(false);
    copies_using_path_with_wildcard_impl(true);
}

fn copies_using_path_with_wildcard_impl(progress: bool) {
    Playground::setup("ucp_test_6", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let src_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls ../formats/*) { open --raw $file.name | to text | hash md5 }"
        )
        .out;

        nu!(
            cwd: dirs.formats(),
            format!(
                "cp {progress_flag} -r ../formats/* {}",
                dirs.test().display()
            )
        );

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
        let dst_hashes = nu!(
            cwd: dirs.formats(),
            format!(
                r#"
                    for file in (ls {}) {{
                        open --raw $file.name
                        | to text
                        | hash md5
                    }}
                "#,
                dirs.test().display()
            )
        )
        .out;
        assert_eq!(src_hashes, dst_hashes);
    })
}

#[test]
fn copies_using_a_glob() {
    copies_using_a_glob_impl(false);
    copies_using_a_glob_impl(true);
}

fn copies_using_a_glob_impl(progress: bool) {
    Playground::setup("ucp_test_7", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        // Get the hash of the file content to check integrity after copy.
        let src_hashes = nu!(
            cwd: dirs.formats(),
            "for file in (ls *) { open --raw $file.name | to text | hash md5 }"
        )
        .out;

        nu!(
            cwd: dirs.formats(),
            format!(
                "cp {progress_flag} -r * {}",
                dirs.test().display()
            )
        );

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
        let dst_hashes = nu!(
            cwd: dirs.formats(),
            format!(
                r#"
                    for file in (ls {}) {{
                        open --raw $file.name
                        | to text
                        | hash md5
                    }}
                "#,
                dirs.test().display()
            )
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
    Playground::setup("ucp_test_8", |dirs, _| {
        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.root(),
            format!(
                "cp {progress_flag} `{}` ucp_test_8/sample.ini",
                dirs.formats().join("sample.ini").display()
            )
        );

        nu!(
            cwd: dirs.root(),
            format!(
                "cp {progress_flag} `{}` ucp_test_8/sample.ini",
                dirs.formats().join("sample.ini").display()
            )
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
    Playground::setup("ucp_test_9", |dirs, sandbox| {
        sandbox.within("foo").within("bar").with_files(&[
            EmptyFile("jtjson"),
            EmptyFile("andres.xml"),
            EmptyFile("yehuda.yaml"),
            EmptyFile("kevin.txt"),
            EmptyFile("many_more.ppl"),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test().join("foo/bar"),
            format!("cp {progress_flag} * ...")
        );

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
    Playground::setup("ucp_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("hello_there")]);
        sandbox.mkdir("hello_again");
        sandbox.within("foo").mkdir("bar");

        let progress_flag = if progress { "-p" } else { "" };

        nu!(
            cwd: dirs.test().join("foo/bar"),
            format!("cp {progress_flag} -r .../hello* .")

        );

        let expected = dirs.test().join("foo/bar");

        assert!(files_exist_at(&["hello_there", "hello_again"], expected));
    })
}

// error msg changes on coreutils
#[test]
fn copy_to_non_existing_dir() {
    copy_to_non_existing_dir_impl(false);
    copy_to_non_existing_dir_impl(true);
}

fn copy_to_non_existing_dir_impl(progress: bool) {
    Playground::setup("ucp_test_11", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("empty_file")]);
        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} empty_file ~/not_a_dir{PATH_SEPARATOR}")
        );
        assert!(actual.err.contains("is not a directory"));
    });
}

#[test]
fn copy_dir_contains_symlink_ignored() {
    copy_dir_contains_symlink_ignored_impl(false);
    copy_dir_contains_symlink_ignored_impl(true);
}

fn copy_dir_contains_symlink_ignored_impl(progress: bool) {
    Playground::setup("ucp_test_12", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            format!("rm {progress_flag} tmp_dir/good_bye; cp -r tmp_dir tmp_dir_2")
        );

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` don't exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(&["hello_there"], expected));
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
    Playground::setup("ucp_test_13", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            format!("rm tmp_dir/good_bye; cp {progress_flag} -r -n tmp_dir tmp_dir_2")
        );

        // check hello_there exists inside `tmp_dir_2`, and `dangle_symlink` also exists inside `tmp_dir_2`.
        let expected = sandbox.cwd().join("tmp_dir_2");
        assert!(files_exist_at(&["hello_there"], expected.clone()));
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
    Playground::setup("ucp_test_14", |_dirs, sandbox| {
        sandbox
            .within("tmp_dir")
            .with_files(&[EmptyFile("hello_there"), EmptyFile("good_bye")])
            .within("tmp_dir")
            .symlink("good_bye", "dangle_symlink");

        let progress_flag = if progress { "-p" } else { "" };

        // make symbolic link and copy.
        nu!(
            cwd: sandbox.cwd(),
            format!(r#"
                rm tmp_dir/good_bye
                cp {progress_flag} -r -n tmp_dir tmp_dir_2
                rm -r tmp_dir
                cp {progress_flag} -r -n tmp_dir_2 tmp_dir
                echo hello_data | save tmp_dir/good_bye
            "#),
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
    Playground::setup("ucp_test_15", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("same.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: dirs.test(),
            format!("cp {progress_flag} same.txt same.txt"),
        );

        let msg = format!(
            "'{}' and '{}' are the same file",
            dirs.test().join("same.txt").display(),
            dirs.test().join("same.txt").display(),
        );
        // debug messages in CI
        if !actual.err.contains(&msg) {
            panic!("stderr was: {}", actual.err);
        }
    });
}

#[test]
#[ignore = "File name in progress bar not on uutils impl"]
fn copy_ignores_ansi() {
    copy_ignores_ansi_impl(false);
    copy_ignores_ansi_impl(true);
}

fn copy_ignores_ansi_impl(progress: bool) {
    Playground::setup("ucp_test_16", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("ls | find test | get name | cp {progress_flag} $in.0 success.txt; ls | find success | get name | ansi strip | get 0"),
        );
        assert_eq!(actual.out, "success.txt");
    });
}

//apparently on windows error msg is different, but linux(where i test) is fine.
//fix later FIXME
#[cfg(unix)]
#[test]
fn copy_file_not_exists_dst() {
    copy_file_not_exists_dst_impl(false);
    copy_file_not_exists_dst_impl(true);
}

#[cfg(unix)]
fn copy_file_not_exists_dst_impl(progress: bool) {
    Playground::setup("ucp_test_17", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("valid.txt")]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} valid.txt ~/invalid_dir/invalid_dir1"),
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
    Playground::setup("ucp_test_18", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithPermission("invalid_prem.txt", false),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} valid.txt invalid_prem.txt"),
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
    Playground::setup("ucp_test_19", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} ucp_test_19/{TEST_HELLO_WORLD_DEST}",
                src.display(),
            )
        );

        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_existing_target() {
    Playground::setup("ucp_test_20", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let existing = dirs.fixtures.join("cp").join(TEST_EXISTING_FILE);

        // Get the hash of the file content to check integrity after copy.
        let src_hash = get_file_hash(src.display());

        // Copy existing file to destination, so that it exists for the test
        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} ucp_test_20/{TEST_EXISTING_FILE}",
                existing.display(),
            )
        );

        // At this point the src and existing files should be different
        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Now for the test
        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} ucp_test_20/{TEST_EXISTING_FILE}",
                src.display(),
            )
        );

        assert!(dirs.test().join(TEST_EXISTING_FILE).exists());

        // Get the hash of the copied file content to check against first_hash.
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_EXISTING_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_multiple_files() {
    Playground::setup("ucp_test_21", |dirs, sandbox| {
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
            format!(
                "cp {} {} ucp_test_21/{TEST_COPY_TO_FOLDER}",
                src1.display(),
                src2.display(),
            )
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
fn test_cp_recurse() {
    Playground::setup("ucp_test_22", |dirs, sandbox| {
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
            format!("cp -r {TEST_COPY_FROM_FOLDER} ucp_test_22/{TEST_COPY_TO_FOLDER_NEW}"),
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_NEW_FILE).display());
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_with_dirs() {
    Playground::setup("ucp_test_23", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());

        //Create target directory
        sandbox.mkdir(TEST_COPY_TO_FOLDER);
        // Start test
        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} ucp_test_23/{TEST_COPY_TO_FOLDER}",
                src.display(),
            )
        );
        let after_cp_hash = get_file_hash(dirs.test().join(TEST_COPY_TO_FOLDER_FILE).display());
        assert_eq!(src_hash, after_cp_hash);

        // Other way around
        sandbox.mkdir(TEST_COPY_FROM_FOLDER);
        let src2 = dirs.fixtures.join("cp").join(TEST_COPY_FROM_FOLDER_FILE);
        let src2_hash = get_file_hash(src2.display());
        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} ucp_test_23/{TEST_HELLO_WORLD_DEST}",
                src2.display(),
            )
        );
        let after_cp_2_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        assert_eq!(src2_hash, after_cp_2_hash);
    });
}
#[cfg(not(windows))]
#[test]
fn test_cp_arg_force() {
    Playground::setup("ucp_test_24", |dirs, sandbox| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let src_hash = get_file_hash(src.display());
        sandbox.with_files(&[FileWithPermission("invalid_prem.txt", false)]);

        nu!(
            cwd: dirs.root(),
            format!(
                "cp {} --force ucp_test_24/{}",
                src.display(),
                "invalid_prem.txt"
            )
        );
        let after_cp_hash = get_file_hash(dirs.test().join("invalid_prem.txt").display());
        // Check content was copied by the use of --force
        assert_eq!(src_hash, after_cp_hash);
    });
}

#[test]
fn test_cp_directory_to_itself_disallowed() {
    Playground::setup("ucp_test_25", |dirs, sandbox| {
        sandbox.mkdir("d");
        let actual = nu!(
            cwd: dirs.root(),
            format!(
                "cp -r ucp_test_25/{}  ucp_test_25/{}",
                "d",
                "d"
            )
        );
        actual
            .err
            .contains("cannot copy a directory, 'd', into itself, 'd/d'");
    });
}

#[test]
fn test_cp_nested_directory_to_itself_disallowed() {
    Playground::setup("ucp_test_26", |dirs, sandbox| {
        sandbox.mkdir("a");
        sandbox.mkdir("a/b");
        sandbox.mkdir("a/b/c");
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "cp -r {} {}",
                "a/b",
                "a/b/c"
            )
        );
        actual
            .err
            .contains("cannot copy a directory, 'a/b', into itself, 'a/b/c/b'");
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_same_file_force() {
    Playground::setup("ucp_test_27", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("f")]);
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "cp --force {} {}",
                "f",
                "f"
            )
        );
        actual.err.contains("cp: 'f' and 'f' are the same file");
        assert!(!dirs.test().join("f~").exists());
    });
}

#[test]
fn test_cp_arg_no_clobber() {
    Playground::setup("ucp_test_28", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let target = dirs.fixtures.join("cp").join(TEST_HOW_ARE_YOU_SOURCE);
        let target_hash = get_file_hash(target.display());

        let _ = nu!(
            cwd: dirs.root(),
            format!(
                "cp {} {} --no-clobber",
                src.display(),
                target.display()
            )
        );
        let after_cp_hash = get_file_hash(target.display());
        // Check content was not clobbered
        assert_eq!(after_cp_hash, target_hash);
    });
}

#[test]
fn test_cp_arg_no_clobber_twice() {
    Playground::setup("ucp_test_29", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("source.txt", "fake data"),
            FileWithContent("source_with_body.txt", "some-body"),
        ]);
        nu!(
            cwd: dirs.root(),
            format!(
                "cp --no-clobber ucp_test_29/{} ucp_test_29/{}",
                "source.txt",
                "dest.txt"
            )
        );
        assert!(dirs.test().join("dest.txt").exists());

        nu!(
            cwd: dirs.root(),
            format!(
                "cp --no-clobber ucp_test_29/{} ucp_test_29/{}",
                "source_with_body.txt",
                "dest.txt"
            )
        );
        // Should have same contents of original empty file as --no-clobber should not overwrite dest.txt
        assert_eq!(file_contents(dirs.test().join("dest.txt")), "fake data");
    });
}

#[test]
fn test_cp_debug_default() {
    Playground::setup("ucp_test_30", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual = nu!(
            cwd: dirs.root(),
            format!(
                "cp --debug {} ucp_test_30/{TEST_HELLO_WORLD_DEST}",
                src.display(),
            )
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
            .contains("copy offload: yes, reflink: unsupported, sparse detection: no")
        {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }

        #[cfg(target_os = "freebsd")]
        if !actual.out.contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported",
        ) {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }

        #[cfg(windows)]
        if !actual.out.contains(
            "copy offload: unsupported, reflink: unsupported, sparse detection: unsupported",
        ) {
            panic!("{}", format!("Failure: stdout was \n{}", actual.out));
        }
        // assert!(actual.out.contains("cp-debug-copy-offload"));
    });
}

#[test]
fn test_cp_verbose_default() {
    Playground::setup("ucp_test_31", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);

        let actual = nu!(
            cwd: dirs.root(),
            format!(
                "cp --verbose {} {TEST_HELLO_WORLD_DEST}",
                src.display(),
            )
        );
        assert!(
            actual.out.contains(
                format!(
                    "'{}' -> '{}'",
                    src.display(),
                    dirs.root().join(TEST_HELLO_WORLD_DEST).display()
                )
                .as_str(),
            )
        );
    });
}

#[test]
fn test_cp_only_source_no_dest() {
    Playground::setup("ucp_test_32", |dirs, _| {
        let src = dirs.fixtures.join("cp").join(TEST_HELLO_WORLD_SOURCE);
        let actual = nu!(
            cwd: dirs.root(),
            format!(
                "cp {}",
                src.display(),
            )
        );
        assert!(
            actual
                .err
                .contains("Missing destination path operand after")
        );
        assert!(actual.err.contains(TEST_HELLO_WORLD_SOURCE));
    });
}

#[test]
fn test_cp_with_vars() {
    Playground::setup("ucp_test_33", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("input")]);
        nu!(
        cwd: dirs.test(),
        "let src = 'input'; let dst = 'target'; cp $src $dst",
        );
        assert!(dirs.test().join("target").exists());
    });
}

#[test]
fn test_cp_destination_after_cd() {
    Playground::setup("ucp_test_34", |dirs, sandbox| {
        sandbox.mkdir("test");
        sandbox.with_files(&[EmptyFile("test/file.txt")]);
        nu!(
        cwd: dirs.test(),
            // Defining variable avoid path expansion of cp argument.
            // If argument was not expanded ucp wrapper should do it
        "cd test; let file = 'copy.txt'; cp file.txt $file",
        );
        assert!(dirs.test().join("test").join("copy.txt").exists());
    });
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn copies_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("ucp_test_34", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        // -- open command doesn't like file name
        //// Get the hash of the file content to check integrity after copy.
        //let src_hash = get_file_hash(src.display());

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "cp '{}' {TEST_HELLO_WORLD_DEST}",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());
        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        //// Get the hash of the copied file content to check against first_hash.
        //let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        //assert_eq!(src_hash, after_cp_hash);
    });
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn copies_files_with_glob_metachars_when_input_are_variables(#[case] src_name: &str) {
    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            src_name,
            "What is the sound of one hand clapping?",
        )]);

        let src = dirs.test().join(src_name);

        // -- open command doesn't like file name
        //// Get the hash of the file content to check integrity after copy.
        //let src_hash = get_file_hash(src.display());

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "let f = '{}'; cp $f {TEST_HELLO_WORLD_DEST}",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());
        assert!(dirs.test().join(TEST_HELLO_WORLD_DEST).exists());

        //// Get the hash of the copied file content to check against first_hash.
        //let after_cp_hash = get_file_hash(dirs.test().join(TEST_HELLO_WORLD_DEST).display());
        //assert_eq!(src_hash, after_cp_hash);
    });
}

#[cfg(not(windows))]
#[rstest]
#[case(r#"'a]?c'"#)]
#[case(r#"'a*.?c'"#)]
// windows doesn't allow filename with `*`.
fn copies_files_with_glob_metachars_nw(#[case] src_name: &str) {
    copies_files_with_glob_metachars(src_name);
    copies_files_with_glob_metachars_when_input_are_variables(src_name);
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_timestamps() {
    // Preserve timestamp and mode

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
        let actual = nu!(
        cwd: dirs.test(),
        "
            chmod +x file.txt
            cp --preserve [ mode timestamps ] file.txt other.txt

            let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
            let new_attrs = ls -l other.txt | get 0 | select mode accessed modified

            $old_attrs == $new_attrs
        ",
        );
        assert!(actual.err.is_empty());
        assert_eq!(actual.out, "true");
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_only_timestamps() {
    // Preserve timestamps and discard all other attributes including mode

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
        let actual = nu!(
        cwd: dirs.test(),
        "
            chmod +x file.txt
            cp --preserve [ timestamps ] file.txt other.txt

            let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
            let new_attrs = ls -l other.txt | get 0 | select mode accessed modified

            print (($old_attrs | select mode) != ($new_attrs | select mode))
            print (($old_attrs | select accessed modified) == ($new_attrs | select accessed modified))
        ",
        );
        assert!(actual.err.is_empty());
        assert_eq!(actual.out, "truetrue");
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_preserve_nothing() {
    // Preserve no attributes

    Playground::setup("ucp_test_35", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("file.txt")]);
        let actual = nu!(
        cwd: dirs.test(),
        "
            chmod +x file.txt
            cp --preserve [] file.txt other.txt

            let old_attrs = ls -l file.txt | get 0 | select mode accessed modified
            let new_attrs = ls -l other.txt | get 0 | select mode accessed modified

            $old_attrs != $new_attrs
        ",
        );
        assert!(actual.err.is_empty());
        assert_eq!(actual.out, "true");
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
            "cp test_file.txt ../",
        );

        assert!(actual.err.is_empty());
        assert!(files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        assert!(files_exist_at(&["test_file.txt"], dirs.test()));
    });
}

#[cfg(not(windows))]
#[test]
fn test_cp_to_customized_home_directory() {
    Playground::setup("cp_to_home", |dirs, sandbox| {
        unsafe {
            std::env::set_var("HOME", dirs.test());
        }
        sandbox.with_files(&[EmptyFile("test_file.txt")]);
        let actual = nu!(cwd: dirs.test(), "mkdir test; cp test_file.txt ~/test/");

        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["test_file.txt"], dirs.test().join("test")));
    })
}

#[test]
fn cp_with_tilde() {
    Playground::setup("cp_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);
        sandbox.within("~tilde2");
        // cp directory
        let actual = nu!(
            cwd: dirs.test(),
            "let f = '~tilde'; cp -r $f '~tilde2'; ls '~tilde2/~tilde' | length"
        );
        assert_eq!(actual.out, "3");

        // cp file
        let actual = nu!(cwd: dirs.test(), "cp '~tilde/f1.txt' ./");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde/f2.txt'; cp $f ./");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));
        assert!(files_exist_at(&["f1.txt"], dirs.test()));
    })
}

#[test]
fn copy_file_with_update_flag() {
    copy_file_with_update_flag_impl(false);
    copy_file_with_update_flag_impl(true);
}

fn copy_file_with_update_flag_impl(progress: bool) {
    Playground::setup("cp_test_36", |_dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("valid.txt"),
            FileWithContent("newer_valid.txt", "body"),
        ]);

        let progress_flag = if progress { "-p" } else { "" };

        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} -u valid.txt newer_valid.txt; open newer_valid.txt"),
        );
        assert!(actual.out.contains("body"));

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(&[FileWithContent("newest_valid.txt", "newest_body")]);
        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} -u newest_valid.txt valid.txt; open valid.txt"),
        );
        assert_eq!(actual.out, "newest_body");

        // when destination doesn't exist
        let actual = nu!(
            cwd: sandbox.cwd(),
            format!("cp {progress_flag} -u newest_valid.txt des_missing.txt; open des_missing.txt"),
        );
        assert_eq!(actual.out, "newest_body");
    });
}

#[test]
fn cp_with_cd() {
    Playground::setup("cp_test_37", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(&[FileWithContent("tmp_dir/file.txt", "body")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            r#"do { cd tmp_dir; let f = 'file.txt'; cp $f .. }; open file.txt"#,
        );
        assert!(actual.out.contains("body"));
    });
}
