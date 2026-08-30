use nu_test_support::prelude::*;
use rstest::rstest;
#[cfg(not(windows))]
use std::fs;
#[cfg(not(windows))]
use std::path::Path;
#[cfg(windows)]
use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

#[cfg(not(windows))]
const RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[test]
fn removes_a_file(playground: Playground) -> Result {
    playground.empty_file("i_will_be_deleted.txt")?;

    let () = test()
        .cwd(playground.path())
        .run("rm i_will_be_deleted.txt")?;

    let path = playground.path().join("i_will_be_deleted.txt");

    assert!(!path.exists());
    Ok(())
}

#[test]
fn removes_files_with_wildcard(playground: Playground) -> Result {
    playground.at("src", |src| {
        src.empty_file("cli.rs")?;
        src.empty_file("lib.rs")?;
        src.empty_file("prelude.rs")?;
        src.at("parser", |parser| {
            parser.empty_file("parse.rs")?;
            parser.empty_file("parser.rs")?;
            parser.empty_file("parse/token_tree.rs")?;
            parser.empty_file("hir/baseline_parse.rs")?;
            parser.empty_file("hir/baseline_parse_tokens.rs")
        })
    })?;

    let () = test().cwd(playground.path()).run("rm src/*/*/*.rs")?;

    assert!(
        !playground
            .path()
            .join("src/parser/parse/token_tree.rs")
            .exists()
    );
    assert!(
        !playground
            .path()
            .join("src/parser/hir/baseline_parse.rs")
            .exists()
    );
    assert!(
        !playground
            .path()
            .join("src/parser/hir/baseline_parse_tokens.rs")
            .exists()
    );

    test()
        .cwd(playground.path())
        .run("glob 'src/*/*/*.rs' | is-empty")
        .expect_value_eq(true)
}

#[test]
fn removes_deeply_nested_directories_with_wildcard_and_recursive_flag(
    playground: Playground,
) -> Result {
    playground.at("src", |src| {
        src.empty_file("cli.rs")?;
        src.empty_file("lib.rs")?;
        src.empty_file("prelude.rs")?;
        src.at("parser", |parser| {
            parser.empty_file("parse.rs")?;
            parser.empty_file("parser.rs")?;
            parser.empty_file("parse/token_tree.rs")?;
            parser.empty_file("hir/baseline_parse.rs")?;
            parser.empty_file("hir/baseline_parse_tokens.rs")
        })
    })?;

    let () = test().cwd(playground.path()).run("rm -r src/*")?;

    assert!(!playground.path().join("src/parser/parse").exists());
    assert!(!playground.path().join("src/parser/hir").exists());
    Ok(())
}

#[test]
fn removes_directory_contents_without_recursive_flag_if_empty(playground: Playground) -> Result {
    let path = playground.path().to_path_buf();
    let () = test().run_with_data("let path = $in; rm $path", path.clone())?;

    assert!(!path.exists());
    Ok(())
}

#[test]
fn removes_directory_contents_with_recursive_flag(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;

    let path = playground.path().to_path_buf();
    let () = test().run_with_data("let path = $in; rm $path --recursive", path.clone())?;

    assert!(!path.exists());
    Ok(())
}

#[test]
fn errors_if_attempting_to_delete_a_directory_with_content_without_recursive_flag(
    playground: Playground,
) -> Result {
    playground.empty_file("some_empty_file.txt")?;
    let err = test()
        .run_with_data("let path = $in; rm $path", playground.path().to_path_buf())
        .expect_shell_error()?;

    assert!(playground.path().exists());
    assert_contains("try --recursive", err.to_string());
    Ok(())
}

#[test]
fn errors_if_attempting_to_delete_home(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("$env.HOME = 'myhome' ; rm -rf ~")
        .expect_shell_error()?;

    assert_contains("You are trying to remove your home dir", err.to_string());
    Ok(())
}

#[test]
fn errors_if_attempting_to_delete_single_dot_as_argument(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("rm .")
        .expect_shell_error()?;

    assert_contains("Cannot remove any parent directory", err.to_string());
    Ok(())
}

#[test]
fn errors_if_attempting_to_delete_two_dot_as_argument(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("rm ..")
        .expect_shell_error()?;

    assert_contains("Cannot", err.to_string());
    Ok(())
}

#[test]
fn removes_multiple_directories(playground: Playground) -> Result {
    playground.at("src", |src| {
        src.empty_file("a.rs")?;
        src.empty_file("b.rs")?;
        src.empty_file("cli/c.rs")?;
        src.empty_file("cli/d.rs")
    })?;
    playground.empty_file("test/a_test.rs")?;
    playground.empty_file("test/b_test.rs")?;

    let () = test()
        .cwd(playground.path())
        .run("rm src test --recursive")?;

    test()
        .cwd(playground.path())
        .run("ls | is-empty")
        .expect_value_eq(true)
}

#[test]
fn removes_multiple_files(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jttxt")?;
    playground.empty_file("andres.txt")?;

    let () = test()
        .cwd(playground.path())
        .run("rm yehuda.txt jttxt andres.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | is-empty")
        .expect_value_eq(true)
}

#[test]
fn removes_multiple_files_with_asterisks(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jt.txt")?;
    playground.empty_file("andres.toml")?;

    let () = test().cwd(playground.path()).run("rm *.txt *.toml")?;

    test()
        .cwd(playground.path())
        .run("ls | is-empty")
        .expect_value_eq(true)
}

#[test]
fn allows_doubly_specified_file(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jt.toml")?;

    let () = test()
        .cwd(playground.path())
        .run("rm *.txt yehuda* *.toml")?;

    test()
        .cwd(playground.path())
        .run("ls | is-empty")
        .expect_value_eq(true)
}

#[test]
fn remove_files_from_two_parents_up_using_multiple_dots_and_glob(playground: Playground) -> Result {
    playground.empty_file("yehuda.txt")?;
    playground.empty_file("jt.txt")?;
    playground.empty_file("kevin.txt")?;
    playground.dir("foo/bar")?;

    let () = test()
        .cwd(playground.path().join("foo/bar"))
        .run("rm .../*.txt")?;

    assert!(!playground.path().join("yehuda.txt").exists());
    assert!(!playground.path().join("jttxt").exists());
    assert!(!playground.path().join("kevin.txt").exists());
    Ok(())
}

#[test]
fn no_errors_if_attempting_to_delete_non_existent_file_with_f_flag(
    playground: Playground,
) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("rm -f non_existent_file.txt")?;
    Ok(())
}

#[test]
fn rm_wildcard_keeps_dotfiles(playground: Playground) -> Result {
    playground.empty_file("foo")?;
    playground.empty_file(".bar")?;

    let () = test().cwd(playground.path()).run("rm *")?;

    assert!(!playground.path().join("foo").exists());
    assert!(playground.path().join(".bar").exists());
    Ok(())
}

#[test]
fn rm_wildcard_leading_dot_deletes_dotfiles(playground: Playground) -> Result {
    playground.empty_file("foo")?;
    playground.empty_file(".bar")?;

    let () = test().cwd(playground.path()).run("rm .b*")?;

    assert!(playground.path().join("foo").exists());
    assert!(!playground.path().join(".bar").exists());
    Ok(())
}

#[test]
fn removes_files_with_case_sensitive_glob_matches_by_default(playground: Playground) -> Result {
    playground.empty_file("A0")?;
    playground.empty_file("a1")?;

    let () = test().cwd(playground.path()).run("rm A*")?;

    let deleted_path = playground.path().join("A0");
    let skipped_path = playground.path().join("a1");

    assert!(!deleted_path.exists());
    assert!(skipped_path.exists());
    Ok(())
}

#[test]
fn remove_ignores_ansi(playground: Playground) -> Result {
    playground.empty_file("test.txt")?;

    test()
        .cwd(playground.path())
        .run("ls | find test | get name | rm $in.0; ls | is-empty")
        .expect_value_eq(true)
}

#[test]
fn removes_symlink(playground: Playground) -> Result {
    let symlink_target = "symlink_target";
    let symlink = "symlink";
    playground.empty_file(symlink_target)?;
    playground.symlink(symlink_target, symlink)?;

    let () = test().cwd(playground.path()).run("rm symlink")?;

    assert!(!playground.path().join(symlink).exists());
    Ok(())
}

#[test]
fn removes_symlink_pointing_to_directory(playground: Playground) -> Result {
    playground.dir("test")?;
    playground.symlink("test", "test_link")?;

    let () = test().cwd(playground.path()).run("rm test_link")?;

    assert!(!playground.path().join("test_link").exists());
    // The pointed directory should not be deleted.
    assert!(playground.path().join("test").exists());
    Ok(())
}

#[test]
fn removes_broken_symlink(playground: Playground) -> Result {
    let symlink_target = "symlink_target_does_not_exist";
    let symlink = "symlink";
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(
        playground.path().join(symlink_target),
        playground.path().join(symlink),
    )?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        playground.path().join(symlink_target),
        playground.path().join(symlink),
    )?;

    let () = test().cwd(playground.path()).run("rm symlink")?;

    assert!(!playground.path().join(symlink).exists());
    Ok(())
}

#[test]
fn removes_file_after_cd(playground: Playground) -> Result {
    playground.empty_file("delete.txt")?;

    let () = test()
        .cwd(playground.path())
        .run("let file = 'delete.txt'; rm $file")?;

    let path = playground.path().join("delete.txt");
    assert!(!path.exists());
    Ok(())
}

#[cfg(not(windows))]
struct Cleanup<'a> {
    dir_to_clean: &'a Path,
}

#[cfg(not(windows))]
fn set_dir_read_only(directory: &Path, read_only: bool) {
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
fn rm_prints_filenames_on_error(playground: Playground) -> Result {
    let file_names = vec!["test1.txt", "test2.txt"];
    for file_name in &file_names {
        playground.empty_file(file_name)?;
    }

    let test_dir = playground.path();

    set_dir_read_only(test_dir, true);
    let _cleanup = Cleanup {
        dir_to_clean: test_dir,
    };

    // This rm is expected to fail, and stderr output indicating so is also expected.
    let result: CompleteResult = test().cwd(test_dir).run_with_data(RUNNER, "rm test*.txt")?;

    assert!(
        file_names
            .iter()
            .all(|file_name| test_dir.join(file_name).exists())
    );
    for file_name in file_names {
        assert_contains("nu::shell::io::permission_denied", &result.stderr);
        assert_contains(file_name, &result.stderr);
    }

    Ok(())
}

#[test]
fn rm_files_inside_glob_metachars_dir(playground: Playground) -> Result {
    let sub_dir = "test[]";
    playground.empty_file("test[]/test_file.txt")?;

    let () = test()
        .cwd(playground.path().join(sub_dir))
        .run("rm test_file.txt")?;
    assert!(
        !playground
            .path()
            .join(sub_dir)
            .join("test_file.txt")
            .exists()
    );
    Ok(())
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn rm_files_with_glob_metachars(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    playground.empty_file(src_name)?;

    let src = playground.path().join(src_name);

    let () = test()
        .cwd(playground.path())
        .run(format!("rm '{}'", src.display()))?;
    assert!(!src.exists());

    // test with variables
    playground.empty_file(src_name)?;
    let () = test()
        .cwd(playground.path())
        .run(format!("let f = '{}'; rm $f", src.display()))?;
    assert!(!src.exists());
    Ok(())
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn rm_files_with_glob_metachars_nw(
    #[ignore] playground: Playground,
    #[case] src_name: &str,
) -> Result {
    rm_files_with_glob_metachars(playground, src_name)
}

#[test]
fn force_rm_suppress_error(playground: Playground) -> Result {
    playground.empty_file("test_file.txt")?;

    // the second rm should suppress error.
    let () = test()
        .cwd(playground.path())
        .run("rm test_file.txt; rm -f test_file.txt")?;
    Ok(())
}

#[test]
fn rm_verbose_returns_deleted_record(playground: Playground) -> Result {
    playground.empty_file("test_file.txt")?;

    let code = "
            let result = (rm -v test_file.txt | first)
            [$result.deleted, ($result.error == null), ($result.path | path basename)]
        ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(test_value!([true, true, "test_file.txt"]))?;
    assert!(!playground.path().join("test_file.txt").exists());
    Ok(())
}

#[test]
fn rm_verbose_returns_error_record_without_failing_pipeline(playground: Playground) -> Result {
    playground.empty_file("present.txt")?;

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
        .cwd(playground.path())
        .run(code)
        .expect_value_eq(test_value!([
            true,
            false,
            true,
            "present.txt",
            "missing.txt",
        ]))?;
    assert!(!playground.path().join("present.txt").exists());
    Ok(())
}

#[test]
fn rm_with_tilde(playground: Playground) -> Result {
    playground.empty_file("~tilde/f1.txt")?;
    playground.empty_file("~tilde/f2.txt")?;
    playground.empty_file("~tilde/f3.txt")?;

    let () = test().cwd(playground.path()).run("rm '~tilde/f1.txt'")?;
    assert!(!playground.path().join("~tilde/f1.txt").exists());

    // pass variable
    let () = test()
        .cwd(playground.path())
        .run("let f = '~tilde/f2.txt'; rm $f")?;
    assert!(!playground.path().join("~tilde/f2.txt").exists());

    // remove directory
    let () = test()
        .cwd(playground.path())
        .run("let f = '~tilde'; rm -r $f")?;
    assert!(!playground.path().join("~tilde").exists());
    Ok(())
}

#[test]
#[cfg(windows)]
fn rm_already_in_use(playground: Playground) -> Result {
    playground.empty_file("i_will_be_used.txt")?;

    let file_path = playground.path().join("i_will_be_used.txt");
    let _file = OpenOptions::new()
        .read(true)
        .write(false)
        .share_mode(0) // deny all sharing
        .open(file_path)?;

    let err = test()
        .cwd(playground.path())
        .run("rm i_will_be_used.txt")
        .expect_shell_error()?;

    assert_contains("AlreadyInUse", format!("{err:?}"));
    Ok(())
}

#[test]
#[exp(nu_experimental::DC_GLOB)]
fn removes_literal_directory_with_recursive_flag(playground: Playground) -> Result {
    playground.empty_file("subdir/test.txt")?;

    let () = test().cwd(playground.path()).run("rm subdir --recursive")?;

    assert!(!playground.path().join("subdir").exists());
    Ok(())
}
