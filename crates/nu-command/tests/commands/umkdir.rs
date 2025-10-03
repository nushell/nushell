use nu_test_support::fs::files_exist_at;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[cfg(not(windows))]
use uucore::mode;

#[test]
fn creates_directory() {
    Playground::setup("mkdir_test_1", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir my_new_directory"
        );

        let expected = dirs.test().join("my_new_directory");

        assert!(expected.exists());
    })
}

#[test]
fn accepts_and_creates_directories() {
    Playground::setup("mkdir_test_2", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir dir_1 dir_2 dir_3"
        );

        assert!(files_exist_at(&["dir_1", "dir_2", "dir_3"], dirs.test()));
    })
}

#[test]
fn creates_intermediary_directories() {
    Playground::setup("mkdir_test_3", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir some_folder/another/deeper_one"
        );

        let expected = dirs.test().join("some_folder/another/deeper_one");

        assert!(expected.exists());
    })
}

#[test]
fn create_directory_two_parents_up_using_multiple_dots() {
    Playground::setup("mkdir_test_4", |dirs, sandbox| {
        sandbox.within("foo").mkdir("bar");

        nu!(
            cwd: dirs.test().join("foo/bar"),
            "mkdir .../boo"
        );

        let expected = dirs.test().join("boo");

        assert!(expected.exists());
    })
}

#[test]
fn print_created_paths() {
    Playground::setup("mkdir_test_2", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), "mkdir -v dir_1 dir_2 dir_3");

        assert!(files_exist_at(&["dir_1", "dir_2", "dir_3"], dirs.test()));

        assert!(actual.out.contains("dir_1"));
        assert!(actual.out.contains("dir_2"));
        assert!(actual.out.contains("dir_3"));
    })
}

#[test]
fn creates_directory_three_dots() {
    Playground::setup("mkdir_test_1", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir test..."
        );

        let expected = dirs.test().join("test...");

        assert!(expected.exists());
    })
}

#[test]
fn creates_directory_four_dots() {
    Playground::setup("mkdir_test_1", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir test...."
        );

        let expected = dirs.test().join("test....");

        assert!(expected.exists());
    })
}

#[test]
fn creates_directory_three_dots_quotation_marks() {
    Playground::setup("mkdir_test_1", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir 'test...'"
        );

        let expected = dirs.test().join("test...");

        assert!(expected.exists());
    })
}

#[test]
fn respects_cwd() {
    Playground::setup("mkdir_respects_cwd", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir 'some_folder'; cd 'some_folder'; mkdir 'another/deeper_one'"
        );

        let expected = dirs.test().join("some_folder/another/deeper_one");

        assert!(expected.exists());
    })
}

#[cfg(not(windows))]
#[test]
fn mkdir_umask_permission() {
    use std::{fs, os::unix::fs::PermissionsExt};

    Playground::setup("mkdir_umask_permission", |dirs, _| {
        nu!(
            cwd: dirs.test(),
            "mkdir test_umask_permission"
        );
        let actual = fs::metadata(dirs.test().join("test_umask_permission"))
            .unwrap()
            .permissions()
            .mode();

        let umask = mode::get_umask();
        let default_mode = 0o40777;
        let expected: u32 = default_mode & !umask;

        assert_eq!(
            actual, expected,
            "Umask should have been applied to created folder"
        );
    })
}

#[test]
fn mkdir_with_tilde() {
    Playground::setup("mkdir with tilde", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), "mkdir '~tilde'");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["~tilde"], dirs.test()));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde2'; mkdir $f");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["~tilde2"], dirs.test()));
    })
}
