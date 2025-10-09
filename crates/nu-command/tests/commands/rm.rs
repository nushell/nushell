#[cfg(not(windows))]
use nu_path::AbsolutePath;
use nu_test_support::fs::{Stub::EmptyFile, files_exist_at};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use rstest::rstest;
#[cfg(not(windows))]
use std::fs;
#[cfg(windows)]
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

#[test]
fn removes_a_file() {
    Playground::setup("rm_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("i_will_be_deleted.txt")]);

        nu!(
            cwd: dirs.root(),
            "rm rm_test_1/i_will_be_deleted.txt"
        );

        let path = dirs.test().join("i_will_be_deleted.txt");

        assert!(!path.exists());
    })
}

#[test]
fn removes_files_with_wildcard() {
    Playground::setup("rm_test_2", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(&[
                EmptyFile("cli.rs"),
                EmptyFile("lib.rs"),
                EmptyFile("prelude.rs"),
            ])
            .within("src/parser")
            .with_files(&[EmptyFile("parse.rs"), EmptyFile("parser.rs")])
            .within("src/parser/parse")
            .with_files(&[EmptyFile("token_tree.rs")])
            .within("src/parser/hir")
            .with_files(&[
                EmptyFile("baseline_parse.rs"),
                EmptyFile("baseline_parse_tokens.rs"),
            ]);

        nu!(
            cwd: dirs.test(),
            r#"rm src/*/*/*.rs"#
        );

        assert!(!files_exist_at(
            &[
                "src/parser/parse/token_tree.rs",
                "src/parser/hir/baseline_parse.rs",
                "src/parser/hir/baseline_parse_tokens.rs"
            ],
            dirs.test()
        ));

        assert_eq!(
            Playground::glob_vec(&format!("{}/src/*/*/*.rs", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_deeply_nested_directories_with_wildcard_and_recursive_flag() {
    Playground::setup("rm_test_3", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(&[
                EmptyFile("cli.rs"),
                EmptyFile("lib.rs"),
                EmptyFile("prelude.rs"),
            ])
            .within("src/parser")
            .with_files(&[EmptyFile("parse.rs"), EmptyFile("parser.rs")])
            .within("src/parser/parse")
            .with_files(&[EmptyFile("token_tree.rs")])
            .within("src/parser/hir")
            .with_files(&[
                EmptyFile("baseline_parse.rs"),
                EmptyFile("baseline_parse_tokens.rs"),
            ]);

        nu!(
            cwd: dirs.test(),
            "rm -r src/*"
        );

        assert!(!files_exist_at(
            &["src/parser/parse", "src/parser/hir"],
            dirs.test()
        ));
    })
}

#[test]
fn removes_directory_contents_without_recursive_flag_if_empty() {
    Playground::setup("rm_test_4", |dirs, _| {
        nu!(
            cwd: dirs.root(),
            "rm rm_test_4"
        );

        assert!(!dirs.test().exists());
    })
}

#[test]
fn removes_directory_contents_with_recursive_flag() {
    Playground::setup("rm_test_5", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        nu!(
            cwd: dirs.root(),
            "rm rm_test_5 --recursive"
        );

        assert!(!dirs.test().exists());
    })
}

#[test]
fn errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag() {
    Playground::setup("rm_test_6", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("some_empty_file.txt")]);
        let actual = nu!(
            cwd: dirs.root(),
            "rm rm_test_6"
        );

        assert!(dirs.test().exists());
        assert!(actual.err.contains("cannot remove non-empty directory"));
    })
}

#[test]
fn errors_if_attempting_to_delete_home() {
    Playground::setup("rm_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "$env.HOME = 'myhome' ; rm -rf ~"
        );

        assert!(actual.err.contains("please use -I or -i"));
    })
}

#[test]
fn errors_if_attempting_to_delete_single_dot_as_argument() {
    Playground::setup("rm_test_7", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm ."
        );

        assert!(actual.err.contains("cannot remove any parent directory"));
    })
}

#[test]
fn errors_if_attempting_to_delete_two_dot_as_argument() {
    Playground::setup("rm_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm .."
        );

        assert!(actual.err.contains("cannot"));
    })
}

#[test]
fn removes_multiple_directories() {
    Playground::setup("rm_test_9", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(&[EmptyFile("a.rs"), EmptyFile("b.rs")])
            .within("src/cli")
            .with_files(&[EmptyFile("c.rs"), EmptyFile("d.rs")])
            .within("test")
            .with_files(&[EmptyFile("a_test.rs"), EmptyFile("b_test.rs")]);

        nu!(
            cwd: dirs.test(),
            "rm src test --recursive"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_multiple_files() {
    Playground::setup("rm_test_10", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "rm yehuda.txt jttxt andres.txt"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn removes_multiple_files_with_asterisks() {
    Playground::setup("rm_test_11", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jt.txt"),
            EmptyFile("andres.toml"),
        ]);

        nu!(
            cwd: dirs.test(),
            "rm *.txt *.toml"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
    })
}

#[test]
fn allows_doubly_specified_file() {
    Playground::setup("rm_test_12", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("yehuda.txt"), EmptyFile("jt.toml")]);

        let actual = nu!(
            cwd: dirs.test(),
            "rm *.txt yehuda* *.toml"
        );

        assert_eq!(
            Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        assert!(!actual.out.contains("error"))
    })
}

#[test]
fn remove_files_from_two_parents_up_using_multiple_dots_and_glob() {
    Playground::setup("rm_test_13", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jt.txt"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        nu!(
            cwd: dirs.test().join("foo/bar"),
            "rm .../*.txt"
        );

        assert!(!files_exist_at(
            &["yehuda.txt", "jttxt", "kevin.txt"],
            dirs.test()
        ));
    })
}

#[test]
fn no_errors_if_attempting_to_delete_non_existent_file_with_f_flag() {
    Playground::setup("rm_test_14", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            "rm -f non_existent_file.txt"
        );

        assert!(!actual.err.contains("no valid path"));
    })
}

#[test]
fn rm_wildcard_keeps_dotfiles() {
    Playground::setup("rm_test_15", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("foo"), EmptyFile(".bar")]);

        nu!(
            cwd: dirs.test(),
            r#"rm *"#
        );

        assert!(!files_exist_at(&["foo"], dirs.test()));
        assert!(files_exist_at(&[".bar"], dirs.test()));
    })
}

#[test]
fn rm_wildcard_leading_dot_deletes_dotfiles() {
    Playground::setup("rm_test_16", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("foo"), EmptyFile(".bar")]);

        nu!(
            cwd: dirs.test(),
            "rm .*"
        );

        assert!(files_exist_at(&["foo"], dirs.test()));
        assert!(!files_exist_at(&[".bar"], dirs.test()));
    })
}

#[test]
fn removes_files_with_case_sensitive_glob_matches_by_default() {
    Playground::setup("glob_test", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("A0"), EmptyFile("a1")]);

        nu!(
            cwd: dirs.root(),
            "rm glob_test/A*"
        );

        let deleted_path = dirs.test().join("A0");
        let skipped_path = dirs.test().join("a1");

        assert!(!deleted_path.exists());
        assert!(skipped_path.exists());
    })
}

#[test]
fn remove_ignores_ansi() {
    Playground::setup("rm_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        let actual = nu!(
            cwd: sandbox.cwd(),
            "ls | find test | get name | rm $in.0; ls | is-empty",
        );
        assert_eq!(actual.out, "true");
    });
}

#[test]
fn removes_symlink() {
    let symlink_target = "symlink_target";
    let symlink = "symlink";
    Playground::setup("rm_test_symlink", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(symlink_target)]);

        #[cfg(not(windows))]
        std::os::unix::fs::symlink(dirs.test().join(symlink_target), dirs.test().join(symlink))
            .unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            dirs.test().join(symlink_target),
            dirs.test().join(symlink),
        )
        .unwrap();

        let _ = nu!(cwd: sandbox.cwd(), "rm symlink");

        assert!(!dirs.test().join(symlink).exists());
    });
}

#[test]
fn removes_symlink_pointing_to_directory() {
    Playground::setup("rm_symlink_to_directory", |dirs, sandbox| {
        sandbox.mkdir("test").symlink("test", "test_link");

        nu!(cwd: sandbox.cwd(), "rm test_link");

        assert!(!dirs.test().join("test_link").exists());
        // The pointed directory should not be deleted.
        assert!(dirs.test().join("test").exists());
    });
}

#[test]
fn removes_file_after_cd() {
    Playground::setup("rm_after_cd", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("delete.txt")]);

        nu!(
            cwd: dirs.root(),
            "let file = 'delete.txt'; cd rm_after_cd; rm $file",
        );

        let path = dirs.test().join("delete.txt");
        assert!(!path.exists());
    })
}

#[cfg(not(windows))]
struct Cleanup<'a> {
    dir_to_clean: &'a AbsolutePath,
}

#[cfg(not(windows))]
fn set_dir_read_only(directory: &AbsolutePath, read_only: bool) {
    let mut permissions = fs::metadata(directory).unwrap().permissions();
    permissions.set_readonly(read_only);
    fs::set_permissions(directory, permissions).expect("failed to set directory permissions");
}

#[cfg(not(windows))]
impl Drop for Cleanup<'_> {
    /// Restores write permissions to the given directory so that the Playground can be successfully
    /// cleaned up.
    fn drop(&mut self) {
        set_dir_read_only(self.dir_to_clean, false);
    }
}

#[test]
// This test is only about verifying file names are included in rm error messages. It is easier
// to only have this work on non-windows systems (i.e., unix-like) than to try to get the
// permissions to work on all platforms.
#[cfg(not(windows))]
fn rm_prints_filenames_on_error() {
    Playground::setup("rm_prints_filenames_on_error", |dirs, sandbox| {
        let file_names = vec!["test1.txt", "test2.txt"];

        let with_files: Vec<_> = file_names
            .iter()
            .map(|file_name| EmptyFile(file_name))
            .collect();
        sandbox.with_files(&with_files);

        let test_dir = dirs.test();

        set_dir_read_only(test_dir, true);
        let _cleanup = Cleanup {
            dir_to_clean: test_dir,
        };

        // This rm is expected to fail, and stderr output indicating so is also expected.
        let actual = nu!(cwd: test_dir, "rm test*.txt");

        assert!(files_exist_at(&file_names, test_dir));
        for file_name in file_names {
            assert!(actual.err.contains("nu::shell::io::permission_denied"));
            assert!(actual.err.contains(file_name));
        }
    });
}

#[test]
fn rm_files_inside_glob_metachars_dir() {
    Playground::setup("rm_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[EmptyFile("test_file.txt")]);

        let actual = nu!(
            cwd: dirs.test().join(sub_dir),
            "rm test_file.txt",
        );

        assert!(actual.err.is_empty());
        assert!(!files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
    });
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn rm_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("rm_files_with_glob_metachars", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(src_name)]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "rm '{}'",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());
        assert!(!src.exists());

        // test with variables
        sandbox.with_files(&[EmptyFile(src_name)]);
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "let f = '{}'; rm $f",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());
        assert!(!src.exists());
    });
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn rm_files_with_glob_metachars_nw(#[case] src_name: &str) {
    rm_files_with_glob_metachars(src_name);
}

#[test]
fn force_rm_suppress_error() {
    Playground::setup("force_rm_suppress_error", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);

        // the second rm should suppress error.
        let actual = nu!(
            cwd: dirs.test(),
            "rm test_file.txt; rm -f test_file.txt",
        );

        assert!(actual.err.is_empty());
    });
}

#[test]
fn rm_with_tilde() {
    Playground::setup("rm_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "rm '~tilde/f1.txt'");
        assert!(actual.err.is_empty());
        assert!(!files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde/f2.txt'; rm $f");
        assert!(actual.err.is_empty());
        assert!(!files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));

        // remove directory
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde'; rm -r $f");
        assert!(actual.err.is_empty());
        assert!(!files_exist_at(&["~tilde"], dirs.test()));
    })
}

#[test]
#[cfg(windows)]
fn rm_already_in_use() {
    Playground::setup("rm_already_in_use", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("i_will_be_used.txt")]);

        let file_path = dirs.root().join("rm_already_in_use/i_will_be_used.txt");
        let _file = OpenOptions::new()
            .read(true)
            .write(false)
            .share_mode(0) // deny all sharing
            .open(file_path)
            .unwrap();

        let outcome = nu!(
            cwd: dirs.root(),
            "rm rm_already_in_use/i_will_be_used.txt"
        );

        assert!(outcome.err.contains("nu::shell::io::already_in_use"));
    })
}
