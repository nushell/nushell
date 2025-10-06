use nu_path::Path;
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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

#[test]
fn filesystem_change_from_current_directory_using_relative_path() {
    Playground::setup("cd_test_1", |dirs, _| {
        let actual = nu!( cwd: dirs.root(), "cd cd_test_1; $env.PWD");

        assert_eq!(Path::new(&actual.out), dirs.test());
    })
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_with_trailing_slash() {
    Playground::setup("cd_test_1_slash", |dirs, _| {
        // Intentionally not using correct path sep because this should work on Windows
        let actual = nu!( cwd: dirs.root(), "cd cd_test_1_slash/; $env.PWD");

        assert_eq!(Path::new(&actual.out), *dirs.test());
    })
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path() {
    Playground::setup("cd_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                r#"
                    cd '{}'
                    $env.PWD
                "#,
                dirs.formats().display()
            )
        );

        assert_eq!(Path::new(&actual.out), dirs.formats());
    })
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path_with_trailing_slash() {
    Playground::setup("cd_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                r#"
                    cd '{}{}'
                    $env.PWD
                "#,
                dirs.formats().display(),
                std::path::MAIN_SEPARATOR_STR,
            )
        );

        assert_eq!(Path::new(&actual.out), dirs.formats());
    })
}

#[test]
fn filesystem_switch_back_to_previous_working_directory() {
    Playground::setup("cd_test_3", |dirs, sandbox| {
        sandbox.mkdir("odin");

        let actual = nu!(
            cwd: dirs.test().join("odin"),
            format!(
                "
                    cd {}
                    cd -
                    $env.PWD
                ",
                dirs.test().display()
            )
        );

        assert_eq!(Path::new(&actual.out), dirs.test().join("odin"));
    })
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_and_dash() {
    Playground::setup("cd_test_4", |dirs, sandbox| {
        sandbox.within("odin").mkdir("-");

        let actual = nu!(
            cwd: dirs.test(),
            "
                cd odin/-
                $env.PWD
            "
        );

        assert_eq!(Path::new(&actual.out), dirs.test().join("odin").join("-"));
    })
}

#[test]
fn filesystem_change_current_directory_to_parent_directory() {
    Playground::setup("cd_test_5", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                cd ..
                $env.PWD
            "
        );

        assert_eq!(Path::new(&actual.out), *dirs.root());
    })
}

#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots() {
    Playground::setup("cd_test_6", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            "
                cd ...
                $env.PWD
            "
        );

        assert_eq!(Path::new(&actual.out), *dirs.test());
    })
}

#[test]
fn filesystem_change_to_home_directory() {
    Playground::setup("cd_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "
                cd ~
                $env.PWD
            "
        );

        assert_eq!(Path::new(&actual.out), dirs::home_dir().unwrap());
    })
}

#[test]
fn filesystem_change_to_a_directory_containing_spaces() {
    Playground::setup("cd_test_9", |dirs, sandbox| {
        sandbox.mkdir("robalino turner katz");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd "robalino turner katz"
                $env.PWD
            "#
        );

        assert_eq!(
            Path::new(&actual.out),
            dirs.test().join("robalino turner katz")
        );
    })
}

#[test]
fn filesystem_not_a_directory() {
    Playground::setup("cd_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("ferris_did_it.txt")]);

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
            actual.err.contains("nu::shell::io::not_a_directory"),
            "actual={:?}",
            actual.err
        );
    })
}

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
            actual.err.contains("nu::shell::io::directory_not_found"),
            "actual={:?}",
            actual.err
        );
    })
}

#[test]
fn filesystem_change_directory_to_symlink_relative() {
    Playground::setup("cd_test_12", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("boo");
        sandbox.symlink("foo", "foo_link");

        let actual = nu!(
            cwd: dirs.test().join("boo"),
            "
                cd ../foo_link
                $env.PWD
            "
        );
        assert_eq!(Path::new(&actual.out), dirs.test().join("foo_link"));

        let actual = nu!(
            cwd: dirs.test().join("boo"),
            "
                cd -P ../foo_link
                $env.PWD
            "
        );
        assert_eq!(Path::new(&actual.out), dirs.test().join("foo"));
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
        assert!(
            dirs.test()
                .join("test_folder")
                .join("test_file.txt")
                .exists()
        );
    })
}

#[cfg(unix)]
#[test]
fn cd_permission_denied_folder() {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let actual = nu!(
            cwd: dirs.test(),
            "
                chmod -x banned
                cd banned
            "
        );
        assert!(actual.err.contains("nu::shell::io::permission_denied"));
        nu!(
            cwd: dirs.test(),
            "
                chmod +x banned
                rm banned
            "
        );
    });
}
// FIXME: cd_permission_denied_folder on windows
#[ignore]
#[cfg(windows)]
#[test]
fn cd_permission_denied_folder() {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let actual = nu!(
            cwd: dirs.test(),
            r"
                icacls banned /deny BUILTIN\Administrators:F
                cd banned
            "
        );
        assert!(actual.err.contains("Folder is not able to read"));
    });
}

#[test]
#[cfg(unix)]
fn pwd_recovery() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let tmpdir = std::env::temp_dir().join("foobar").display().to_string();

    // We `cd` into a temporary directory, then spawn another `nu` process to
    // delete that directory. Then we attempt to recover by running `cd /`.
    let cmd = format!("mkdir {tmpdir}; cd {tmpdir}; {nu} -c 'cd /; rm -r {tmpdir}'; cd /; pwd");
    let actual = nu!(cmd);

    assert_eq!(actual.out, "/");
}
