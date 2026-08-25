use nu_protocol::shell_error;
use nu_test_support::prelude::*;

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
#[test]
fn cd_bare_drive_on_current_drive_stays(playground: Playground) -> Result {
    let drive = drive_letter(playground.path());
    for input in [
        format!("cd {}:", drive.to_ascii_lowercase()),
        format!("cd {}:", drive.to_ascii_uppercase()),
        format!("cd \"{drive}:	\""),
        format!("cd -P {drive}:"),
    ] {
        test()
            .cwd(playground.path())
            .run(format!("{input}; $env.PWD"))
            .expect_value_eq(playground.path())?;
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn cd_drive_relative_path_on_current_drive(playground: Playground) -> Result {
    playground.dir("foo")?;
    let drive = drive_letter(playground.path());

    test()
        .cwd(playground.path())
        .run(format!("cd {drive}:foo; $env.PWD"))
        .expect_value_eq(playground.path().join("foo"))?;

    test()
        .cwd(playground.path().join("foo"))
        .run(format!("cd {drive}:..; $env.PWD"))
        .expect_value_eq(playground.path())
}

#[cfg(windows)]
#[test]
<<<<<<< HEAD
fn cd_missing_drive_reports_input() -> Result {
    let Some(drive) = ('A'..='Z')
        .rev()
        .find(|letter| !std::path::Path::new(&format!("{letter}:\\")).exists())
    else {
        return Ok(());
    };
    let err = test().run(format!("cd {drive}:")).expect_io_error()?;
    assert_eq!(err.path.unwrap().to_string_lossy(), format!("{drive}:"));
    assert!(matches!(
        err.kind,
        nu_protocol::shell_error::io::ErrorKind::DirectoryNotFound
    ));
    Ok(())
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path(playground: Playground) -> Result {
<<<<<<< HEAD
    let destination = playground.dir("cd_test_1")?;
    test()
        .cwd(playground.path())
        .run("cd cd_test_1; $env.PWD")
        .expect_value_eq(destination)
=======
    playground.dir("cd_test_1")?;
    test()
        .cwd(playground.path())
        .run("cd cd_test_1; $env.PWD")
        .expect_value_eq(playground.path().join("cd_test_1"))
>>>>>>> 4f4a9394c (fixed some warnings)
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_with_trailing_slash(
    playground: Playground,
) -> Result {
<<<<<<< HEAD
    let destination = playground.dir("cd_test_1_slash")?;
=======
fn filesystem_change_from_current_directory_using_relative_path_with_trailing_slash(
    playground: Playground,
) -> Result {
>>>>>>> 926ed3f27 (fmt)
=======
    playground.dir("cd_test_1_slash")?;
>>>>>>> 4f4a9394c (fixed some warnings)
    // Intentionally not using correct path sep because this should work on Windows
    test()
        .cwd(playground.path())
        .run("cd cd_test_1_slash/; $env.PWD")
<<<<<<< HEAD
        .expect_value_eq(destination)
=======
        .expect_value_eq(playground.path().join("cd_test_1_slash"))
>>>>>>> 4f4a9394c (fixed some warnings)
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path(playground: Playground) -> Result {
<<<<<<< HEAD
    let destination = playground.dir("destination")?;
    test()
        .cwd(playground.path())
        .run_with_data("cd $in; $env.PWD", destination.as_path())
        .expect_value_eq(destination)
=======
    let formats = FIXTURES.join("formats");
    test()
        .cwd(playground.path())
        .run_with_data("cd $in; $env.PWD", formats.clone())
        .expect_value_eq(formats)
>>>>>>> 4f4a9394c (fixed some warnings)
}

#[test]
fn filesystem_change_from_current_directory_using_absolute_path_with_trailing_slash(
    playground: Playground,
) -> Result {
<<<<<<< HEAD
<<<<<<< HEAD
    let destination = playground.dir("destination")?;
    let mut dir = destination.to_string_lossy().into_owned();
=======
    let mut dir = dirs.formats().to_string_lossy().into_owned();
>>>>>>> 926ed3f27 (fmt)
=======
    let formats = FIXTURES.join("formats");
    let mut dir = formats.to_string_lossy().into_owned();
>>>>>>> 4f4a9394c (fixed some warnings)
    // Keep this portable: Windows expects `\` while Unix expects `/`.
    if !dir.ends_with(std::path::MAIN_SEPARATOR) {
        dir.push(std::path::MAIN_SEPARATOR);
    }

    test()
        .cwd(playground.path())
        .run_with_data("cd $in; $env.PWD", dir)
<<<<<<< HEAD
        .expect_value_eq(destination)
=======
        .expect_value_eq(formats)
>>>>>>> 4f4a9394c (fixed some warnings)
}

#[test]
fn filesystem_switch_back_to_previous_working_directory(playground: Playground) -> Result {
    playground.dir("odin")?;
    let odin_path = playground.path().join("odin");

    test()
        .cwd(&odin_path)
        .run_with_data("cd $in; cd -; $env.PWD", playground.path())
        .expect_value_eq(odin_path)
}

#[test]
fn filesystem_change_from_current_directory_using_relative_path_and_dash(
    playground: Playground,
) -> Result {
    playground.dir("odin/-")?;
    let odin_path = playground.path().join("odin").join("-");
<<<<<<< HEAD
=======

>>>>>>> 2c3f367df (update some more tests)
    test()
        .cwd(playground.path())
        .run("cd odin/-; $env.PWD")
        .expect_value_eq(odin_path)
}

#[test]
fn filesystem_change_current_directory_to_parent_directory(playground: Playground) -> Result {
    let child = playground.dir("child")?;
    test()
        .cwd(child)
        .run("cd ..; $env.PWD")
<<<<<<< HEAD
        .expect_value_eq(playground.path())
=======
        .expect_value_eq(playground.path().parent().unwrap())
>>>>>>> 4f4a9394c (fixed some warnings)
}

#[test]
fn filesystem_change_current_directory_to_two_parents_up_using_multiple_dots(
    playground: Playground,
) -> Result {
    playground.dir("foo/bar")?;
<<<<<<< HEAD
=======

>>>>>>> 2c3f367df (update some more tests)
    test()
        .cwd(playground.path().join("foo").join("bar"))
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
    playground.dir("robalino turner katz")?;
    test()
        .cwd(playground.path())
        .run("cd 'robalino turner katz'; $env.PWD")
        .expect_value_eq(playground.path().join("robalino turner katz"))
}

#[test]
fn filesystem_not_a_directory(playground: Playground) -> Result {
    playground.empty_file("ferris_did_it.txt")?;

    let err = test()
        .cwd(playground.path())
        .run("cd ferris_did_it.txt")
        .expect_io_error()?;

    assert_eq!(
        err.path.unwrap(),
        playground.path().join("ferris_did_it.txt")
    );
    assert!(matches!(
        err.kind,
        shell_error::io::ErrorKind::Std(std::io::ErrorKind::NotADirectory, ..)
    ));

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
    assert!(matches!(
        err.kind,
        nu_protocol::shell_error::io::ErrorKind::DirectoryNotFound
    ));

    Ok(())
}

#[test]
fn filesystem_change_directory_to_symlink_relative(playground: Playground) -> Result {
    playground.dir("foo")?;
    playground.dir("boo")?;
    playground.symlink("foo", "foo_link")?;

    test()
        .cwd(playground.path().join("boo"))
        .run("cd ../foo_link; $env.PWD")
        .expect_value_eq(playground.path().join("foo_link"))?;

    test()
        .cwd(playground.path().join("boo"))
        .run("cd -P ../foo_link; $env.PWD")
        .expect_value_eq(playground.path().join("foo"))?;

    Ok(())
}

// FIXME: jt: needs more work
#[ignore]
#[cfg(target_os = "windows")]
#[test]
fn test_change_windows_drive(playground: Playground) -> Result {
    playground.dir("test_folder")?;

    let code = r#"
        subst Z: test_folder
        Z:
        echo "some text" | save test_file.txt
        cd ~
        subst Z: /d
    "#;

    let _: () = test().cwd(playground.path()).run(code)?;
    assert!(
        playground
            .path()
            .join("test_folder")
            .join("test_file.txt")
            .exists()
    );

    Ok(())
}

#[cfg(unix)]
#[test]
fn cd_permission_denied_folder(playground: Playground) -> Result {
    playground.dir("banned")?;
    let code = "
        chmod -x banned
        cd banned
    ";
    let err = test()
        .inherit_path()
        .cwd(playground.path())
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
    let _: () = test().inherit_path().cwd(playground.path()).run(cleanup)?;
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
        .cwd(playground.path())
        .run(code)
        .expect_shell_error()?;
    assert_contains("Folder is not able to read", err.to_string());
    Ok(())
}

#[test]
#[deps(NU)]
#[cfg(unix)]
fn pwd_recovery() -> Result {
    let nu = NU.path().display().to_string();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let tmpdir = std::env::temp_dir()
        .join(format!("nu_pwd_recovery_{}_{}", std::process::id(), unique))
        .display()
        .to_string();

    let ctx = test_record! {
        "tmpdir" => tmpdir,
        "nu" => nu,
    };
    // We `cd` into a temporary directory, then spawn another `nu` process to
    // delete that directory. Then we attempt to recover by running `cd /`.
    let code = r#"
        let ctx = $in

        mkdir $ctx.tmpdir
        cd $ctx.tmpdir
        ^$ctx.nu -c $"cd /; rm -r '($ctx.tmpdir)'"
        cd /
        pwd
    "#;

    test().run_with_data(code, ctx).expect_value_eq("/")
}

#[test]
fn cd_dot_dot_from_deleted_cwd() -> Result {
    Playground::setup("cd_deleted_parent", |dirs, sandbox| {
        sandbox.mkdir("a/b");
        let a = dirs.test().join("a");
        let b = a.join("b");
        std::fs::remove_dir(&b)?;

        test().cwd(&b).run("cd ..; $env.PWD").expect_value_eq(&*a)?;

        // When both parent and child are gone, `cd ..` errors instead of
        // silently resolving from `$nu.home-dir`.
        std::fs::remove_dir(&a)?;
        let err = test().cwd(&b).run("cd ..").expect_io_error()?;
        assert!(matches!(
            err.kind,
            nu_protocol::shell_error::io::ErrorKind::DirectoryNotFound
        ));

        // Traversing high enough to reach an existing ancestor still works.
        test()
            .cwd(&b)
            .run("cd ../..; $env.PWD")
            .expect_value_eq(dirs.test())
    })
}
