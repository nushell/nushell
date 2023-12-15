use std::path::Path;

use nu_test_support::fs::file_contents;
use nu_test_support::fs::{
    files_exist_at, AbsoluteFile,
    Stub::{EmptyFile, FileWithContent, FileWithPermission},
};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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

        assert!(actual.err.contains("../formats"));
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
                Path::new("andres.txt"),
            ],
            &expected_dir,
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
            jts_expected_copied_dir,
        ));
        assert!(files_exist_at(
            vec![Path::new("coverage.txt"), Path::new("commands.txt")],
            andres_expected_copied_dir,
        ));
        assert!(files_exist_at(
            vec![Path::new("defer-evaluation.txt")],
            yehudas_expected_copied_dir,
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
            dirs.test(),
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
            dirs.test(),
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
            dirs.test(),
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

#[ignore = "duplicate test with slight differences in ucp"]
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
        assert!(actual.err.contains("directory not found"));
        assert!(actual.err.contains("does not exist"));
    });
}

#[ignore = "duplicate test with slight differences in ucp"]
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
        assert!(files_exist_at(vec!["hello_there"], expected.clone()));
        let path = expected.join("dangle_symlink");
        assert!(!path.exists() && !path.is_symlink());
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

#[ignore = "duplicate test with slight differences in ucp"]
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
        assert!(actual.err.contains("Copy aborted"));
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

#[ignore = "duplicate test with ucp with slight differences"]
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
            actual.err.contains("invalid_dir1") && actual.err.contains("copying to destination")
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

        assert!(actual.err.contains("invalid_prem.txt") && actual.err.contains("denied"));
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
            "cp {} -u valid.txt newer_valid.txt; open newer_valid.txt",
            progress_flag,
        );
        assert!(actual.out.contains("body"));

        // create a file after assert to make sure that newest_valid.txt is newest
        std::thread::sleep(std::time::Duration::from_secs(1));
        sandbox.with_files(vec![FileWithContent("newest_valid.txt", "newest_body")]);
        let actual = nu!(cwd: sandbox.cwd(), "cp {} -u newest_valid.txt valid.txt; open valid.txt", progress_flag);
        assert_eq!(actual.out, "newest_body");

        // when destination doesn't exist
        let actual = nu!(cwd: sandbox.cwd(), "cp {} -u newest_valid.txt des_missing.txt; open des_missing.txt", progress_flag);
        assert_eq!(actual.out, "newest_body");
    });
}

#[test]
fn cp_with_cd() {
    Playground::setup("cp_test_20", |_dirs, sandbox| {
        sandbox
            .mkdir("tmp_dir")
            .with_files(vec![FileWithContent("tmp_dir/file.txt", "body")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            r#"do { cd tmp_dir; let f = 'file.txt'; cp $f .. }; open file.txt"#,
        );
        assert!(actual.out.contains("body"));
    });
}
