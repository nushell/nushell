#[cfg(not(windows))]
use nu_path::AbsolutePath;
use nu_test_support::fs::{Stub::EmptyFile, files_exist_at};
use nu_test_support::prelude::*;
use rstest::rstest;
#[cfg(not(windows))]
use std::fs;
#[cfg(windows)]
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

#[cfg(not(windows))]
const RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[test]
fn removes_a_file() -> Result {
    Playground::setup("rm_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("i_will_be_deleted.txt")]);

        let () = test()
            .cwd(dirs.root())
            .run("rm rm_test_1/i_will_be_deleted.txt")?;

        let path = dirs.test().join("i_will_be_deleted.txt");

        assert!(!path.exists());
        Ok(())
    })
}

#[test]
fn removes_files_with_wildcard() -> Result {
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

        let () = test().cwd(dirs.test()).run("rm src/*/*/*.rs")?;

        assert!(!files_exist_at(
            &[
                "src/parser/parse/token_tree.rs",
                "src/parser/hir/baseline_parse.rs",
                "src/parser/hir/baseline_parse_tokens.rs"
            ],
            dirs.test()
        ));

        assert_eq!(
            deprecated::Playground::glob_vec(&format!("{}/src/*/*/*.rs", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        Ok(())
    })
}

#[test]
fn removes_deeply_nested_directories_with_wildcard_and_recursive_flag() -> Result {
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

        let () = test().cwd(dirs.test()).run("rm -r src/*")?;

        assert!(!files_exist_at(
            &["src/parser/parse", "src/parser/hir"],
            dirs.test()
        ));
        Ok(())
    })
}

#[test]
fn removes_directory_contents_without_recursive_flag_if_empty() -> Result {
    Playground::setup("rm_test_4", |dirs, _| {
        let () = test().cwd(dirs.root()).run("rm rm_test_4")?;

        assert!(!dirs.test().exists());
        Ok(())
    })
}

#[test]
fn removes_directory_contents_with_recursive_flag() -> Result {
    Playground::setup("rm_test_5", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let () = test().cwd(dirs.root()).run("rm rm_test_5 --recursive")?;

        assert!(!dirs.test().exists());
        Ok(())
    })
}

#[test]
fn errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag() -> Result {
    Playground::setup("rm_test_6", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("some_empty_file.txt")]);
        let err = test()
            .cwd(dirs.root())
            .run("rm rm_test_6")
            .expect_shell_error()?;

        assert!(dirs.test().exists());
        assert_contains("try --recursive", err.to_string());
        Ok(())
    })
}

#[test]
fn errors_if_attempting_to_delete_home() -> Result {
    Playground::setup("rm_test_8", |dirs, _| {
        let err = test()
            .cwd(dirs.root())
            .run("$env.HOME = 'myhome' ; rm -rf ~")
            .expect_shell_error()?;

        assert_contains("You are trying to remove your home dir", err.to_string());
        Ok(())
    })
}

#[test]
fn errors_if_attempting_to_delete_single_dot_as_argument() -> Result {
    Playground::setup("rm_test_7", |dirs, _| {
        let err = test().cwd(dirs.root()).run("rm .").expect_shell_error()?;

        assert_contains("Cannot remove any parent directory", err.to_string());
        Ok(())
    })
}

#[test]
fn errors_if_attempting_to_delete_two_dot_as_argument() -> Result {
    Playground::setup("rm_test_8", |dirs, _| {
        let err = test().cwd(dirs.root()).run("rm ..").expect_shell_error()?;

        assert_contains("Cannot", err.to_string());
        Ok(())
    })
}

#[test]
fn removes_multiple_directories() -> Result {
    Playground::setup("rm_test_9", |dirs, sandbox| {
        sandbox
            .within("src")
            .with_files(&[EmptyFile("a.rs"), EmptyFile("b.rs")])
            .within("src/cli")
            .with_files(&[EmptyFile("c.rs"), EmptyFile("d.rs")])
            .within("test")
            .with_files(&[EmptyFile("a_test.rs"), EmptyFile("b_test.rs")]);

        let () = test().cwd(dirs.test()).run("rm src test --recursive")?;

        assert_eq!(
            deprecated::Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        Ok(())
    })
}

#[test]
fn removes_multiple_files() -> Result {
    Playground::setup("rm_test_10", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let () = test()
            .cwd(dirs.test())
            .run("rm yehuda.txt jttxt andres.txt")?;

        assert_eq!(
            deprecated::Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        Ok(())
    })
}

#[test]
fn removes_multiple_files_with_asterisks() -> Result {
    Playground::setup("rm_test_11", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jt.txt"),
            EmptyFile("andres.toml"),
        ]);

        let () = test().cwd(dirs.test()).run("rm *.txt *.toml")?;

        assert_eq!(
            deprecated::Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        Ok(())
    })
}

#[test]
fn allows_doubly_specified_file() -> Result {
    Playground::setup("rm_test_12", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("yehuda.txt"), EmptyFile("jt.toml")]);

        let () = test().cwd(dirs.test()).run("rm *.txt yehuda* *.toml")?;

        assert_eq!(
            deprecated::Playground::glob_vec(&format!("{}/*", dirs.test().display())),
            Vec::<std::path::PathBuf>::new()
        );
        Ok(())
    })
}

#[test]
fn remove_files_from_two_parents_up_using_multiple_dots_and_glob() -> Result {
    Playground::setup("rm_test_13", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jt.txt"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        let () = test()
            .cwd(dirs.test().join("foo/bar"))
            .run("rm .../*.txt")?;

        assert!(!files_exist_at(
            &["yehuda.txt", "jttxt", "kevin.txt"],
            dirs.test()
        ));
        Ok(())
    })
}

#[test]
fn no_errors_if_attempting_to_delete_non_existent_file_with_f_flag() -> Result {
    Playground::setup("rm_test_14", |dirs, _| {
        let () = test().cwd(dirs.root()).run("rm -f non_existent_file.txt")?;
        Ok(())
    })
}

#[test]
fn rm_wildcard_keeps_dotfiles() -> Result {
    Playground::setup("rm_test_15", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("foo"), EmptyFile(".bar")]);

        let () = test().cwd(dirs.test()).run("rm *")?;

        assert!(!files_exist_at(&["foo"], dirs.test()));
        assert!(files_exist_at(&[".bar"], dirs.test()));
        Ok(())
    })
}

#[test]
fn rm_wildcard_leading_dot_deletes_dotfiles() -> Result {
    Playground::setup("rm_test_16", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("foo"), EmptyFile(".bar")]);

        let () = test().cwd(dirs.test()).run("rm .b*")?;

        assert!(files_exist_at(&["foo"], dirs.test()));
        assert!(!files_exist_at(&[".bar"], dirs.test()));
        Ok(())
    })
}

#[test]
fn removes_files_with_case_sensitive_glob_matches_by_default() -> Result {
    Playground::setup("glob_test", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("A0"), EmptyFile("a1")]);

        let () = test().cwd(dirs.root()).run("rm glob_test/A*")?;

        let deleted_path = dirs.test().join("A0");
        let skipped_path = dirs.test().join("a1");

        assert!(!deleted_path.exists());
        assert!(skipped_path.exists());
        Ok(())
    })
}

#[test]
fn remove_ignores_ansi() -> Result {
    Playground::setup("rm_test_ansi", |_dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test.txt")]);

        test()
            .cwd(sandbox.cwd())
            .run("ls | find test | get name | rm $in.0; ls | is-empty")
            .expect_value_eq(true)
    })
}

#[test]
fn removes_symlink() -> Result {
    let symlink_target = "symlink_target";
    let symlink = "symlink";
    Playground::setup("rm_test_symlink", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(symlink_target)]);

        #[cfg(not(windows))]
        std::os::unix::fs::symlink(dirs.test().join(symlink_target), dirs.test().join(symlink))?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            dirs.test().join(symlink_target),
            dirs.test().join(symlink),
        )?;

        let () = test().cwd(sandbox.cwd()).run("rm symlink")?;

        assert!(!dirs.test().join(symlink).exists());
        Ok(())
    })
}

#[test]
fn removes_symlink_pointing_to_directory() -> Result {
    Playground::setup("rm_symlink_to_directory", |dirs, sandbox| {
        sandbox.mkdir("test").symlink("test", "test_link");

        let () = test().cwd(sandbox.cwd()).run("rm test_link")?;

        assert!(!dirs.test().join("test_link").exists());
        // The pointed directory should not be deleted.
        assert!(dirs.test().join("test").exists());
        Ok(())
    })
}

#[test]
fn removes_broken_symlink() -> Result {
    let symlink_target = "symlink_target_does_not_exist";
    let symlink = "symlink";
    Playground::setup("rm_test_broken_symlink", |dirs, sandbox| {
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(dirs.test().join(symlink_target), dirs.test().join(symlink))?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            dirs.test().join(symlink_target),
            dirs.test().join(symlink),
        )?;

        let () = test().cwd(sandbox.cwd()).run("rm symlink")?;

        assert!(!dirs.test().join(symlink).exists());
        Ok(())
    })
}

#[test]
fn removes_file_after_cd() -> Result {
    Playground::setup("rm_after_cd", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("delete.txt")]);

        let () = test()
            .cwd(dirs.root())
            .run("let file = 'delete.txt'; cd rm_after_cd; rm $file")?;

        let path = dirs.test().join("delete.txt");
        assert!(!path.exists());
        Ok(())
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

// This test is only about verifying file names are included in rm error messages. It is easier
// to only have this work on non-windows systems (i.e., unix-like) than to try to get the
// permissions to work on all platforms.
#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn rm_prints_filenames_on_error() -> Result {
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
        let result: CompleteResult = test().cwd(test_dir).run_with_data(RUNNER, "rm test*.txt")?;

        assert!(files_exist_at(&file_names, test_dir));
        for file_name in file_names {
            assert_contains("nu::shell::io::permission_denied", &result.stderr);
            assert_contains(file_name, &result.stderr);
        }

        Ok(())
    })
}

#[test]
fn rm_files_inside_glob_metachars_dir() -> Result {
    Playground::setup("rm_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[EmptyFile("test_file.txt")]);

        let () = test()
            .cwd(dirs.test().join(sub_dir))
            .run("rm test_file.txt")?;
        assert!(!files_exist_at(
            &["test_file.txt"],
            dirs.test().join(sub_dir)
        ));
        Ok(())
    })
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn rm_files_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("rm_files_with_glob_metachars", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile(src_name)]);

        let src = dirs.test().join(src_name);

        let () = test()
            .cwd(dirs.test())
            .run(format!("rm '{}'", src.display()))?;
        assert!(!src.exists());

        // test with variables
        sandbox.with_files(&[EmptyFile(src_name)]);
        let () = test()
            .cwd(dirs.test())
            .run(format!("let f = '{}'; rm $f", src.display()))?;
        assert!(!src.exists());
        Ok(())
    })
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn rm_files_with_glob_metachars_nw(#[case] src_name: &str) -> Result {
    rm_files_with_glob_metachars(src_name)
}

#[test]
fn force_rm_suppress_error() -> Result {
    Playground::setup("force_rm_suppress_error", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);

        // the second rm should suppress error.
        let () = test()
            .cwd(dirs.test())
            .run("rm test_file.txt; rm -f test_file.txt")?;
        Ok(())
    })
}

#[test]
fn rm_verbose_returns_deleted_record() -> Result {
    Playground::setup("rm_verbose_returns_deleted_record", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("test_file.txt")]);

        let code = "
            let result = (rm -v test_file.txt | first)
            [$result.deleted, ($result.error == null), ($result.path | path basename)]
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(test_value!([true, true, "test_file.txt"]))?;
        assert!(!dirs.test().join("test_file.txt").exists());
        Ok(())
    })
}

#[test]
fn rm_verbose_returns_error_record_without_failing_pipeline() -> Result {
    Playground::setup(
        "rm_verbose_returns_error_record_without_failing_pipeline",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("present.txt")]);

            let code = "
                let result = (rm -v present.txt missing.txt | update path { path basename })
                let present = ($result | where path == present.txt | first)
                let missing = ($result | where path == missing.txt | first)
                [
                    $present.deleted,
                    $missing.deleted,
                    ($missing.error != null),
                    $present.path,
                    $missing.path,
                ]
            ";

            test()
                .cwd(dirs.test())
                .run(code)
                .expect_value_eq(test_value!([
                    true,
                    false,
                    true,
                    "present.txt",
                    "missing.txt",
                ]))?;
            assert!(!dirs.test().join("present.txt").exists());
            Ok(())
        },
    )
}

#[test]
fn rm_with_tilde() -> Result {
    Playground::setup("rm_tilde", |dirs, sandbox| {
        sandbox.within("~tilde").with_files(&[
            EmptyFile("f1.txt"),
            EmptyFile("f2.txt"),
            EmptyFile("f3.txt"),
        ]);

        let () = test().cwd(dirs.test()).run("rm '~tilde/f1.txt'")?;
        assert!(!files_exist_at(&["f1.txt"], dirs.test().join("~tilde")));

        // pass variable
        let () = test()
            .cwd(dirs.test())
            .run("let f = '~tilde/f2.txt'; rm $f")?;
        assert!(!files_exist_at(&["f2.txt"], dirs.test().join("~tilde")));

        // remove directory
        let () = test().cwd(dirs.test()).run("let f = '~tilde'; rm -r $f")?;
        assert!(!files_exist_at(&["~tilde"], dirs.test()));
        Ok(())
    })
}

#[test]
#[cfg(windows)]
fn rm_already_in_use() -> Result {
    Playground::setup("rm_already_in_use", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("i_will_be_used.txt")]);

        let file_path = dirs.root().join("rm_already_in_use/i_will_be_used.txt");
        let _file = OpenOptions::new()
            .read(true)
            .write(false)
            .share_mode(0) // deny all sharing
            .open(file_path)?;

        let err = test()
            .cwd(dirs.root())
            .run("rm rm_already_in_use/i_will_be_used.txt")
            .expect_shell_error()?;

        assert_contains("AlreadyInUse", format!("{err:?}"));
        Ok(())
    })
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn removes_literal_directory_with_recursive_flag() -> Result {
    Playground::setup("rm_literal_dir_dc", |dirs, sandbox| {
        sandbox
            .within("subdir")
            .with_files(&[EmptyFile("test.txt")]);

        let () = test()
            .cwd(dirs.root())
            .run("rm rm_literal_dir_dc/subdir --recursive")?;

        assert!(!dirs.test().join("subdir").exists());
        Ok(())
    })
}
