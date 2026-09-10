use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn creates_directory() -> Result {
    Playground::setup("mkdir_test_1", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir my_new_directory")?;

        let expected = dirs.test().join("my_new_directory");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[test]
fn accepts_and_creates_directories() -> Result {
    Playground::setup("mkdir_test_2", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir dir_1 dir_2 dir_3")?;

        assert!(dirs.test().join("dir_1").is_dir());
        assert!(dirs.test().join("dir_2").is_dir());
        assert!(dirs.test().join("dir_3").is_dir());
        Ok(())
    })
}

#[test]
fn creates_intermediary_directories() -> Result {
    Playground::setup("mkdir_test_3", |dirs, _| {
        let () = test()
            .cwd(dirs.test())
            .run("mkdir some_folder/another/deeper_one")?;

        let expected = dirs.test().join("some_folder/another/deeper_one");

        assert!(expected.is_dir());
        Ok(())
    })
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
fn print_created_paths() -> Result {
    Playground::setup("mkdir_test_2", |dirs, _| {
        let actual: String = test()
            .cwd(dirs.test())
            .run("mkdir -v dir_1 dir_2 dir_3 | to text")?;

        assert!(dirs.test().join("dir_1").is_dir());
        assert!(dirs.test().join("dir_2").is_dir());
        assert!(dirs.test().join("dir_3").is_dir());

        assert_contains("dir_1", &actual);
        assert_contains("dir_2", &actual);
        assert_contains("dir_3", &actual);
        Ok(())
    })
}

#[test]
fn creates_directory_three_dots() -> Result {
    Playground::setup("mkdir_test_1", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir test...")?;

        let expected = dirs.test().join("test...");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[test]
fn creates_directory_four_dots() -> Result {
    Playground::setup("mkdir_test_1", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir test....")?;

        let expected = dirs.test().join("test....");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[test]
fn creates_directory_three_dots_quotation_marks() -> Result {
    Playground::setup("mkdir_test_1", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir 'test...'")?;

        let expected = dirs.test().join("test...");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[test]
fn respects_cwd() -> Result {
    Playground::setup("mkdir_respects_cwd", |dirs, _| {
        let () = test()
            .cwd(dirs.test())
            .run("mkdir 'some_folder'; cd 'some_folder'; mkdir 'another/deeper_one'")?;

        let expected = dirs.test().join("some_folder/another/deeper_one");

        assert!(expected.is_dir());
        Ok(())
    })
}

#[cfg(not(windows))]
#[test]
#[serial]
fn mkdir_umask_permission() -> Result {
    use std::{fs, os::unix::fs::PermissionsExt};

    // Serial: process umask is global; parallel tests that call get_umask/mkdir
    // (uu_mkdir briefly sets umask to 0) can race this assertion.
    Playground::setup("mkdir_umask_permission", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir test_umask_permission")?;
        let actual = fs::metadata(dirs.test().join("test_umask_permission"))
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
    })
}

#[test]
fn mkdir_with_tilde() -> Result {
    Playground::setup("mkdir with tilde", |dirs, _| {
        let () = test().cwd(dirs.test()).run("mkdir '~tilde'")?;
        assert!(dirs.test().join("~tilde").is_dir());

        // pass variable
        let () = test().cwd(dirs.test()).run("let f = '~tilde2'; mkdir $f")?;
        assert!(dirs.test().join("~tilde2").is_dir());
        Ok(())
    })
}

#[test]
fn mkdir_with_interpolation_simple() -> Result {
    Playground::setup("mkdir interpolation simple", |dirs, _| {
        // Test with a simple variable interpolation
        let () = test()
            .cwd(dirs.test())
            .run("let x = 'test'; mkdir xxx/($x)")?;

        assert!(dirs.test().join("xxx/test").is_dir());
        assert!(!dirs.test().join("xxx/($x)").is_dir());
        Ok(())
    })
}

#[test]
fn mkdir_with_interpolation() -> Result {
    Playground::setup("mkdir with interpolation", |dirs, _| {
        // Test with each command using interpolation
        let _: Value = test()
            .cwd(dirs.test())
            .run("[ a b c ] | each { mkdir xxx/($in) }")?;

        assert!(dirs.test().join("xxx/a").is_dir());
        assert!(dirs.test().join("xxx/b").is_dir());
        assert!(dirs.test().join("xxx/c").is_dir());

        // Should not create a literal directory named "($in)"
        assert!(!dirs.test().join("xxx/($in)").is_dir());
        Ok(())
    })
}

#[test]
#[cfg(not(windows))]
fn mkdir_continues_creating_directories_after_error() -> Result {
    Playground::setup("mkdir_continues_after_error", |dirs, _| {
        let _ = test()
            .cwd(dirs.test())
            .run("mkdir before /etc/ack after")
            .expect_error()?;

        assert!(dirs.test().join("before").is_dir());
        assert!(dirs.test().join("after").is_dir());
        Ok(())
    })
}

#[test]
#[cfg(not(windows))]
fn mkdir_verbose_reports_errors_without_failing() -> Result {
    Playground::setup("mkdir_verbose_reports_errors_without_failing", |dirs, _| {
        let _: Value = test()
            .cwd(dirs.test())
            .run("mkdir -v before /etc/ack after")?;

        assert!(dirs.test().join("before").is_dir());
        assert!(dirs.test().join("after").is_dir());
        Ok(())
    })
}
