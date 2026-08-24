use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn creates_directory(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir my_new_directory")?;

    let expected = playground.path().join("my_new_directory");

    assert!(expected.is_dir());
    Ok(())
}

#[test]
fn accepts_and_creates_directories(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir dir_1 dir_2 dir_3")?;

    assert!(playground.path().join("dir_1").is_dir());
    assert!(playground.path().join("dir_2").is_dir());
    assert!(playground.path().join("dir_3").is_dir());
    Ok(())
}

#[test]
fn creates_intermediary_directories(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir some_folder/another/deeper_one")?;

    let expected = playground.path().join("some_folder/another/deeper_one");

    assert!(expected.is_dir());
    Ok(())
}

#[test]
fn create_directory_two_parents_up_using_multiple_dots() -> Result {
    Playground::setup("mkdir_test_4", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        let () = test()
            .cwd(dirs.test().join("foo/bar"))
            .run("mkdir .../boo")?;

        let expected = dirs.test().join("boo");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[test]
fn print_created_paths(playground: Playground) -> Result {
    let actual: String = test()
        .cwd(playground.path())
        .run("mkdir -v dir_1 dir_2 dir_3 | to text")?;

    assert!(playground.path().join("dir_1").is_dir());
    assert!(playground.path().join("dir_2").is_dir());
    assert!(playground.path().join("dir_3").is_dir());

    assert_contains("dir_1", &actual);
    assert_contains("dir_2", &actual);
    assert_contains("dir_3", &actual);
    Ok(())
}

#[test]
fn creates_directory_three_dots(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("mkdir test...")?;

    let expected = playground.path().join("test...");

    assert!(expected.is_dir());
    Ok(())
}

#[test]
fn creates_directory_four_dots(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("mkdir test....")?;

    let expected = playground.path().join("test....");

    assert!(expected.is_dir());
    Ok(())
}

#[test]
fn creates_directory_three_dots_quotation_marks(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("mkdir 'test...'")?;

    let expected = playground.path().join("test...");

    assert!(expected.is_dir());
    Ok(())
}

#[test]
fn respects_cwd(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir 'some_folder'; cd 'some_folder'; mkdir 'another/deeper_one'")?;

    let expected = playground.path().join("some_folder/another/deeper_one");

    assert!(expected.is_dir());
    Ok(())
}

#[cfg(not(windows))]
#[test]
#[serial]
fn mkdir_umask_permission(playground: Playground) -> Result {
    use std::{fs, os::unix::fs::PermissionsExt};

    // Serial: process umask is global; parallel tests that call get_umask/mkdir
    // (uu_mkdir briefly sets umask to 0) can race this assertion.
    let () = test()
        .cwd(playground.path())
        .run("mkdir test_umask_permission")?;
    let actual = fs::metadata(playground.path().join("test_umask_permission"))
        .unwrap()
        .permissions()
        .mode();

    let umask = nu_system::get_umask();
    let default_mode = 0o40777;
    let expected: u32 = default_mode & !umask;

    assert_eq!(
        actual, expected,
        "Umask should have been applied to created folder"
    );
    Ok(())
}

#[test]
fn mkdir_with_tilde(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("mkdir '~tilde'")?;
    assert!(playground.path().join("~tilde").is_dir());

    // pass variable
    let () = test()
        .cwd(playground.path())
        .run("let f = '~tilde2'; mkdir $f")?;
    assert!(playground.path().join("~tilde2").is_dir());
    Ok(())
}

#[test]
fn mkdir_with_interpolation_simple(playground: Playground) -> Result {
    // Test with a simple variable interpolation
    let () = test()
        .cwd(playground.path())
        .run("let x = 'test'; mkdir xxx/($x)")?;

    assert!(playground.path().join("xxx/test").is_dir());
    assert!(!playground.path().join("xxx/($x)").is_dir());
    Ok(())
}

#[test]
fn mkdir_with_interpolation(playground: Playground) -> Result {
    // Test with each command using interpolation
    let _: Value = test()
        .cwd(playground.path())
        .run("[ a b c ] | each { mkdir xxx/($in) }")?;

    assert!(playground.path().join("xxx/a").is_dir());
    assert!(playground.path().join("xxx/b").is_dir());
    assert!(playground.path().join("xxx/c").is_dir());

    // Should not create a literal directory named "($in)"
    assert!(!playground.path().join("xxx/($in)").is_dir());
    Ok(())
}

#[test]
#[cfg(not(windows))]
fn mkdir_continues_creating_directories_after_error(playground: Playground) -> Result {
    let _ = test()
        .cwd(playground.path())
        .run("mkdir before /etc/ack after")
        .expect_error()?;

    assert!(playground.path().join("before").is_dir());
    assert!(playground.path().join("after").is_dir());
    Ok(())
}

#[test]
#[cfg(not(windows))]
fn mkdir_verbose_reports_errors_without_failing(playground: Playground) -> Result {
    let _: Value = test()
        .cwd(playground.path())
        .run("mkdir -v before /etc/ack after")?;

    assert!(playground.path().join("before").is_dir());
    assert!(playground.path().join("after").is_dir());
    Ok(())
}
