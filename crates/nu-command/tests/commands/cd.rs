use nu_protocol::shell_error;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn cd_works_with_in_var(playground: Playground) -> Result {
    playground.dir("cd_test_1")?;
    let code = r#"
        "cd_test_1"
        | cd $in; $env.PWD
        | path split
        | last
    "#;

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("cd_test_1")
}

/// The drive letter `dir` is on, e.g. `C`.
#[cfg(windows)]
fn drive_letter(dir: &std::path::Path) -> char {
    use std::path::{Component, Prefix};
    match dir.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter as char,
            other => panic!("{dir:?} is not on a drive: {other:?}"),
        },
        _ => panic!("{dir:?} has no prefix"),
    }
}

#[cfg(windows)]
#[rstest::rstest]
#[case::lowercase(false, false, false)]
#[case::uppercase(true, false, false)]
#[case::trailing_tab(true, true, false)]
#[case::physical(true, false, true)]
fn cd_bare_drive_on_current_drive_stays(
    #[case] uppercase: bool,
    #[case] trailing_tab: bool,
    #[case] physical: bool,
    #[ignore] playground: Playground,
) -> Result {
    let drive = drive_letter(playground.path());
    let drive = if uppercase {
        drive.to_ascii_uppercase()
    } else {
        drive.to_ascii_lowercase()
    };
    let input = if trailing_tab {
        format!("cd \"{drive}:\t\"")
    } else if physical {
        format!("cd -P {drive}:")
    } else {
        format!("cd {drive}:")
    };

    test()
        .cwd(playground.path())
        .run(format!("{input}; $env.PWD"))
        .expect_value_eq(playground.path())
}

#[expect(clippy::disallowed_names)]
#[cfg(windows)]
#[test]
fn cd_drive_relative_path_on_current_drive(playground: Playground) -> Result {
    let foo = playground.dir("foo")?;
    let drive = drive_letter(playground.path());

    test()
        .cwd(playground.path())
        .run(format!("cd {drive}:foo; $env.PWD"))
        .expect_value_eq(foo.as_path())?;

    test()
        .cwd(foo)
        .run(format!("cd {drive}:..; $env.PWD"))
        .expect_value_eq(playground.path())
}

#[cfg(windows)]
#[test]
fn cd_missing_drive_reports_input() -> Result {
    let Some(drive) = ('A'..='Z')
        .rev()
        .find(|letter| !std::path::Path::new(&format!("{letter}:\\")).exists())
    else {
        return Ok(());
    };
    let err = test().run(format!("cd {drive}:")).expect_io_error()?;
    assert_eq!(err.path.unwrap().to_string_lossy(), format!("{drive}:"));
    assert_matches!(err.kind, shell_error::io::ErrorKind::DirectoryNotFound);
    Ok(())
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path(playground: Playground) -> Result {
    let destination = playground.dir("cd_test_1")?;
    test()
        .cwd(playground.path())
        .run("cd cd_test_1; $env.PWD")
        .expect_value_eq(destination)
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_with_trailing_slash(
    playground: Playground,
) -> Result {
    let destination = playground.dir("cd_test_1_slash")?;
    // intentionally use `/` to check that it also works on Windows
    test()
        .cwd(playground.path())
        .run("cd cd_test_1_slash/; $env.PWD")
        .expect_value_eq(destination)
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path(playground: Playground) -> Result {
    let destination = playground.dir("destination")?;
    test()
        .cwd(playground.path())
        .run_with_data("cd $in; $env.PWD", destination.as_path())
        .expect_value_eq(destination)
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path_with_trailing_slash(
    playground: Playground,
) -> Result {
    let destination = playground.dir("destination")?;
    let mut dir = destination.to_string_lossy().into_owned();
    dir.push(std::path::MAIN_SEPARATOR);

    test()
        .cwd(playground.path())
        .run_with_data("cd $in; $env.PWD", dir)
        .expect_value_eq(destination)
}

#[test]
fn filesystem_switch_back_to_previous_working_directory(playground: Playground) -> Result {
    let odin = playground.dir("odin")?;
    test()
        .cwd(&odin)
        .run_with_data("cd $in; cd -; $env.PWD", playground.path())
        .expect_value_eq(odin)
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_and_dash(
    playground: Playground,
) -> Result {
    playground.dir("odin/-")?;
    test()
        .cwd(playground.path())
        .run("cd odin/-; $env.PWD")
        .expect_value_eq(playground.path().join("odin").join("-"))
}

#[test]
fn filesystem_change_current_directory_to_parent_directory(playground: Playground) -> Result {
    let child = playground.dir("child")?;
    test()
        .cwd(child)
        .run("cd ..; $env.PWD")
        .expect_value_eq(playground.path())
}

#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots(
    playground: Playground,
) -> Result {
    let child = playground.dir("foo/bar")?;
    test()
        .cwd(child)
        .run("cd ...; $env.PWD")
        .expect_value_eq(playground.path())
}

#[test]
fn filesystem_change_to_home_directory(playground: Playground) -> Result {
    test()
        .cwd(playground.path())
        .run("cd ~; $env.PWD")
        .expect_value_eq(dirs::home_dir())
}

#[test]
fn filesystem_change_to_a_directory_containing_spaces(playground: Playground) -> Result {
    let destination = playground.dir("robalino turner katz")?;
    test()
        .cwd(playground.path())
        .run("cd 'robalino turner katz'; $env.PWD")
        .expect_value_eq(destination)
}

#[test]
fn filesystem_not_a_directory(playground: Playground) -> Result {
    let file = playground.empty_file("ferris_did_it.txt")?;
    let err = test()
        .cwd(playground.path())
        .run("cd ferris_did_it.txt")
        .expect_io_error()?;

    assert_eq!(err.path.unwrap(), file);
    assert_matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::NotADirectory, ..)
    );
    Ok(())
}

#[test]
fn filesystem_directory_not_found(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("cd dir_that_does_not_exist")
        .expect_io_error()?;

    assert_eq!(
        err.path.unwrap().to_string_lossy(),
        "dir_that_does_not_exist"
    );
    assert_matches!(err.kind, shell_error::io::ErrorKind::DirectoryNotFound);
    Ok(())
}

#[expect(clippy::disallowed_names)]
#[test]
fn filesystem_change_directory_to_symlink_relative(playground: Playground) -> Result {
    let foo = playground.dir("foo")?;
    let boo = playground.dir("boo")?;
    playground.symlink("foo", "foo_link")?;

    test()
        .cwd(&boo)
        .run("cd ../foo_link; $env.PWD")
        .expect_value_eq(playground.path().join("foo_link"))?;

    #[cfg(not(windows))]
    let foo = foo.canonicalize()?;
    test()
        .cwd(boo)
        .run("cd -P ../foo_link; $env.PWD")
        .expect_value_eq(foo)
}

// FIXME: jt: needs more work
#[ignore]
#[cfg(target_os = "windows")]
#[test]
#[serial]
fn test_change_windows_drive(playground: Playground) -> Result {
    playground.dir("test_folder")?;
    let code = r#"
        subst Z: test_folder
        Z:
        "some text" | save test_file.txt
        cd ~
        subst Z: /d
    "#;

    let () = test().inherit_path().cwd(playground.path()).run(code)?;
    assert_eq!(
        std::fs::read_to_string(playground.path().join("test_folder/test_file.txt"))?,
        "some text"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn cd_permission_denied_folder(playground: Playground) -> Result {
    use std::os::unix::fs::PermissionsExt;

    let banned = playground.dir("banned")?;
    let permissions = std::fs::metadata(&banned)?.permissions();
    std::fs::set_permissions(&banned, std::fs::Permissions::from_mode(0o600))?;
    let result = test()
        .cwd(playground.path())
        .run("cd banned")
        .expect_io_error();
    std::fs::set_permissions(&banned, permissions)?;
    let err = result?;
    assert_matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::PermissionDenied, ..)
    );
    Ok(())
}

// FIXME: cd_permission_denied_folder on windows
#[ignore]
#[cfg(windows)]
#[test]
fn cd_permission_denied_folder(playground: Playground) -> Result {
    playground.dir("banned")?;
    let code = r"
        icacls banned /deny BUILTIN\Administrators:F
        cd banned
    ";
    let err = test()
        .inherit_path()
        .cwd(playground.path())
        .run(code)
        .expect_shell_error()?;
    assert_contains("Folder is not able to read", err.to_string());
    Ok(())
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn pwd_recovery(playground: Playground) -> Result {
    let doomed = playground.dir("doomed")?;
    // a child process deletes the current directory before `cd /` recovers
    let mut tester = test()
        .cwd(playground.path())
        .env("DOOMED", doomed.as_path());
    let () = tester.run_with_data("cd $in", doomed.as_path())?;
    let result: CompleteResult = tester.run("nu -n -c 'cd /; rm -r $env.DOOMED' | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stderr, "");
    assert!(!doomed.exists());

    tester.run("cd /; pwd").expect_value_eq("/")
}

// TODO: fix this test
#[test]
#[ignore = "dot dot navigation maybe broken?"]
fn cd_dot_dot_from_deleted_cwd(playground: Playground) -> Result {
    let b = playground.dir("a/b")?;
    let a = playground.path().join("a");
    std::fs::remove_dir(&b)?;

    test()
        .cwd(&b)
        .run("cd ..; $env.PWD")
        .expect_value_eq(a.as_path())?;

    // with both parent and child gone, `cd ..` must error and not silently resolve from `$nu.home-dir`
    std::fs::remove_dir(&a)?;
    let err = test().cwd(&b).run("cd ..").expect_io_error()?;
    assert_matches!(err.kind, shell_error::io::ErrorKind::DirectoryNotFound);

    // an existing ancestor is still reachable
    test()
        .cwd(&b)
        .run("cd ../..; $env.PWD")
        .expect_value_eq(playground.path())
}
