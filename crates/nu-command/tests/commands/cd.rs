use nu_protocol::shell_error;
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn cd_works_with_in_var() -> Result {
    Playground::setup("cd_test_1", |dirs, _| {
        let code = r#"
            "cd_test_1"
            | cd $in; $env.PWD
            | path split
            | last
        "#;

        test()
            .cwd(dirs.root())
            .run(code)
            .expect_value_eq("cd_test_1")
    })
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path() -> Result {
    Playground::setup("cd_test_1", |dirs, _| {
        test()
            .cwd(dirs.root())
            .run("cd cd_test_1; $env.PWD")
            .expect_value_eq(dirs.test())
    })
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_with_trailing_slash() -> Result {
    Playground::setup("cd_test_1_slash", |dirs, _| {
        // Intentionally not using correct path sep because this should work on Windows
        test()
            .cwd(dirs.root())
            .run("cd cd_test_1_slash/; $env.PWD")
            .expect_value_eq(dirs.test())
    })
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path() -> Result {
    Playground::setup("cd_test_2", |dirs, _| {
        let code = format!("cd '{}'; $env.PWD", dirs.formats().display());
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(dirs.formats())
    })
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path_with_trailing_slash() -> Result {
    Playground::setup("cd_test_2", |dirs, _| {
        let code = format!(
            "cd '{}{}'; $env.PWD",
            dirs.formats().display(),
            std::path::MAIN_SEPARATOR_STR,
        );

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(dirs.formats())
    })
}

#[test]
fn filesystem_switch_back_to_previous_working_directory() -> Result {
    Playground::setup("cd_test_3", |dirs, sandbox| {
        sandbox.mkdir("odin");
        let odin_path = dirs.test().join("odin");

        let code = format!("cd {}; cd -; $env.PWD", dirs.test().display());

        test().cwd(&odin_path).run(code).expect_value_eq(odin_path)
    })
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_and_dash() -> Result {
    Playground::setup("cd_test_4", |dirs, sandbox| {
        sandbox.within("odin").mkdir("-");
        let odin_path = dirs.test().join("odin").join("-");
        test()
            .cwd(dirs.test())
            .run("cd odin/-; $env.PWD")
            .expect_value_eq(odin_path)
    })
}

#[test]
fn filesystem_change_current_directory_to_parent_directory() -> Result {
    Playground::setup("cd_test_5", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run("cd ..; $env.PWD")
            .expect_value_eq(dirs.root())
    })
}

#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots() -> Result {
    Playground::setup("cd_test_6", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");
        test()
            .cwd(dirs.test().join("foo").join("bar"))
            .run("cd ...; $env.PWD")
            .expect_value_eq(dirs.test())
    })
}

#[test]
fn filesystem_change_to_home_directory() -> Result {
    Playground::setup("cd_test_8", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run("cd ~; $env.PWD")
            .expect_value_eq(dirs::home_dir())
    })
}

#[test]
fn filesystem_change_to_a_directory_containing_spaces() -> Result {
    Playground::setup("cd_test_9", |dirs, sandbox| {
        sandbox.mkdir("robalino turner katz");
        test()
            .cwd(dirs.test())
            .run("cd 'robalino turner katz'; $env.PWD")
            .expect_value_eq(dirs.test().join("robalino turner katz"))
    })
}

#[test]
fn filesystem_not_a_directory() -> Result {
    Playground::setup("cd_test_10", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("ferris_did_it.txt")]);

        let err = test()
            .cwd(dirs.test())
            .run("cd ferris_did_it.txt")
            .expect_io_error()?;

        assert_eq!(err.path.unwrap(), dirs.test().join("ferris_did_it.txt"));
        assert!(matches!(
            err.kind,
            shell_error::io::ErrorKind::Std(std::io::ErrorKind::NotADirectory, ..)
        ));

        Ok(())
    })
}

#[test]
fn filesystem_directory_not_found() -> Result {
    Playground::setup("cd_test_11", |dirs, _| {
        let err = test()
            .cwd(dirs.test())
            .run("cd dir_that_does_not_exist")
            .expect_io_error()?;

        assert_eq!(
            err.path.unwrap().to_string_lossy(),
            "dir_that_does_not_exist"
        );
        assert!(matches!(
            err.kind,
            nu_protocol::shell_error::io::ErrorKind::DirectoryNotFound
        ));

        Ok(())
    })
}

#[test]
fn filesystem_change_directory_to_symlink_relative() -> Result {
    Playground::setup("cd_test_12", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("boo");
        sandbox.symlink("foo", "foo_link");

        test()
            .cwd(dirs.test().join("boo"))
            .run("cd ../foo_link; $env.PWD")
            .expect_value_eq(dirs.test().join("foo_link"))?;

        test()
            .cwd(dirs.test().join("boo"))
            .run("cd -P ../foo_link; $env.PWD")
            .expect_value_eq(dirs.test().join("foo"))?;

        Ok(())
    })
}

// FIXME: jt: needs more work
#[ignore]
#[cfg(target_os = "windows")]
#[test]
fn test_change_windows_drive() -> Result {
    Playground::setup("cd_test_20", |dirs, sandbox| {
        sandbox.mkdir("test_folder");

        let code = r#"
            subst Z: test_folder
            Z:
            echo "some text" | save test_file.txt
            cd ~
            subst Z: /d
        "#;

        let _: () = test().cwd(dirs.test()).run(code)?;
        assert!(
            dirs.test()
                .join("test_folder")
                .join("test_file.txt")
                .exists()
        );

        Ok(())
    })
}

#[cfg(unix)]
#[test]
fn cd_permission_denied_folder() -> Result {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let code = "
            chmod -x banned
            cd banned
        ";
        let err = test()
            .inherit_path()
            .cwd(dirs.test())
            .run(code)
            .expect_io_error()?;
        assert!(matches!(
            err.kind,
            nu_protocol::shell_error::io::ErrorKind::Std(std::io::ErrorKind::PermissionDenied, ..)
        ));
        let cleanup = "
            chmod +x banned
            rm banned
        ";
        let _: () = test().inherit_path().cwd(dirs.test()).run(cleanup)?;
        Ok(())
    })
}

// FIXME: cd_permission_denied_folder on windows
#[ignore]
#[cfg(windows)]
#[test]
fn cd_permission_denied_folder() -> Result {
    Playground::setup("cd_test_21", |dirs, sandbox| {
        sandbox.mkdir("banned");
        let code = r"
            icacls banned /deny BUILTIN\Administrators:F
            cd banned
        ";
        let err = test().cwd(dirs.test()).run(code).expect_shell_error()?;
        assert_contains("Folder is not able to read", err.to_string());
        Ok(())
    })
}

#[test]
#[cfg(unix)]
fn pwd_recovery() -> Result {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let tmpdir = std::env::temp_dir()
        .join(format!("nu_pwd_recovery_{}_{}", std::process::id(), unique))
        .display()
        .to_string();

    // We `cd` into a temporary directory, then spawn another `nu` process to
    // delete that directory. Then we attempt to recover by running `cd /`.
    let cmd =
        format!("mkdir '{tmpdir}'; cd '{tmpdir}'; {nu} -c \"cd /; rm -r '{tmpdir}'\"; cd /; pwd");
    test().run(cmd).expect_value_eq("/")
}
