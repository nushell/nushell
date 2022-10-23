use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::path::PathBuf;

#[test]
fn cd_works_with_in_var() {
    Playground::setup("cd_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            r#"
                "cd_test_1" | cd $in; $env.PWD | path split | last
            "#
        );

        assert_eq!("cd_test_1", actual.out);
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_from_current_directory_using_relative_path() {
    Playground::setup("cd_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.root(),
            r#"
                cd cd_test_1
                echo (pwd)
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.test());
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_from_current_directory_using_absolute_path() {
    Playground::setup("cd_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd "{}"
                echo (pwd)
            "#,
            dirs.formats().display()
        );

        assert_eq!(PathBuf::from(actual.out), dirs.formats());
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_switch_back_to_previous_working_directory() {
    Playground::setup("cd_test_3", |dirs, sandbox| {
        sandbox.mkdir("odin");

        let actual = nu!(
            cwd: dirs.test().join("odin"),
            r#"
                cd {}
                cd -
                echo (pwd)
            "#,
            dirs.test().display()
        );

        assert_eq!(PathBuf::from(actual.out), dirs.test().join("odin"));
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesytem_change_from_current_directory_using_relative_path_and_dash() {
    Playground::setup("cd_test_4", |dirs, sandbox| {
        sandbox.within("odin").mkdir("-");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd odin/-
                echo (pwd)
            "#
        );

        assert_eq!(
            PathBuf::from(actual.out),
            dirs.test().join("odin").join("-")
        );
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_current_directory_to_parent_directory() {
    Playground::setup("cd_test_5", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ..
                echo (pwd)
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.root());
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots() {
    Playground::setup("cd_test_6", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                cd ...
                echo (pwd)
            "#
        );

        assert_eq!(PathBuf::from(actual.out), *dirs.test());
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_current_directory_to_parent_directory_after_delete_cwd() {
    Playground::setup("cd_test_7", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                rm {}/foo/bar
                echo ","
                cd ..
                echo (pwd)
            "#,
            dirs.test().display()
        );

        let actual = actual.out.split(',').nth(1).unwrap();

        assert_eq!(PathBuf::from(actual), *dirs.test().join("foo"));
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_to_home_directory() {
    Playground::setup("cd_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd ~
                echo (pwd)
            "#
        );

        assert_eq!(Some(PathBuf::from(actual.out)), dirs_next::home_dir());
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_to_a_directory_containing_spaces() {
    Playground::setup("cd_test_9", |dirs, sandbox| {
        sandbox.mkdir("robalino turner katz");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd "robalino turner katz"
                echo (pwd)
            "#
        );

        assert_eq!(
            PathBuf::from(actual.out),
            dirs.test().join("robalino turner katz")
        );
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_not_a_directory() {
    Playground::setup("cd_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("ferris_did_it.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "cd ferris_did_it.txt"
        );

        assert!(
            actual.err.contains("ferris_did_it.txt"),
            "actual={:?}",
            actual.err
        );
        assert!(
            actual.err.contains("is not a directory"),
            "actual={:?}",
            actual.err
        );
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_directory_not_found() {
    Playground::setup("cd_test_11", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "cd dir_that_does_not_exist"

        );

        assert!(
            actual.err.contains("dir_that_does_not_exist"),
            "actual={:?}",
            actual.err
        );
        assert!(
            actual.err.contains("directory not found"),
            "actual={:?}",
            actual.err
        );
    })
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn filesystem_change_directory_to_symlink_relative() {
    Playground::setup("cd_test_12", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("boo");
        sandbox.symlink("foo", "foo_link");

        let actual = nu!(
            cwd: dirs.test().join("boo"),
            r#"
                cd ../foo_link
                echo (pwd)
            "#
        );

        assert_eq!(PathBuf::from(actual.out), dirs.test().join("foo"));
    })
}

// FIXME: jt: needs more work
#[ignore]
#[cfg(target_os = "windows")]
#[test]
fn test_change_windows_drive() {
    Playground::setup("cd_test_20", |dirs, sandbox| {
        sandbox.mkdir("test_folder");

        let _actual = nu!(
            cwd: dirs.test(),
            r#"
                subst Z: test_folder
                Z:
                echo "some text" | save test_file.txt
                cd ~
                subst Z: /d
            "#
        );
        assert!(dirs
            .test()
            .join("test_folder")
            .join("test_file.txt")
            .exists());
    })
}

#[cfg(unix)]
#[test]
fn cd_permission_deined_folder() {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                chmod -x banned
                cd banned
            "#
        );
        assert!(actual.err.contains("Cannot change directory to"));
        nu!(
            cwd: dirs.test(),
            r#"
                chmod +x banned
                rm banned
            "#
        );
    });
}
// FIXME: cd_permission_deined_folder on windows
#[ignore]
#[cfg(windows)]
#[test]
fn cd_permission_deined_folder() {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                icacls banned /deny BUILTIN\Administrators:F
                cd banned
            "#
        );
        assert!(actual.err.contains("Folder is not able to read"));
    });
}
