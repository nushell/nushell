use nu_test_support::fs::files_exist_at;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};
use std::path::Path;

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

        assert!(files_exist_at(
            vec![Path::new("dir_1"), Path::new("dir_2"), Path::new("dir_3")],
            dirs.test()
        ));
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
        let actual = nu!(
         cwd: dirs.test(),
         pipeline(
             "mkdir -v dir_1 dir_2 dir_3"
        ));

        assert!(files_exist_at(
            vec![Path::new("dir_1"), Path::new("dir_2"), Path::new("dir_3")],
            dirs.test()
        ));

        assert!(actual.err.contains("dir_1"));
        assert!(actual.err.contains("dir_2"));
        assert!(actual.err.contains("dir_3"));
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
fn mkdir_shows_multiple_errors() {
    Playground::setup("mkdir_shows_multiple_errors", |dirs, playground| {
        playground.mkdir("test");

        let test_root = dirs.test();
        let test_dir = test_root.join("test");

        let mut test_dir_permissions = playground.permissions(&test_dir);

        test_dir_permissions.set_readonly(true);
        test_dir_permissions.apply().unwrap();

        let actual = nu!(cwd: test_root, "mkdir test/test1 test/test2 test3");

        assert!(
            actual.err.contains("Could not create some directories"),
            "should show generic error message"
        );

        assert!(
            actual.err.contains(&format!(
                "failed to create directory {}: Permission denied",
                test_dir.join("test1").to_string_lossy()
            )),
            "permissions error"
        );
        assert!(
            actual.err.contains(&format!(
                "failed to create directory {}: Permission denied",
                test_dir.join("test2").to_string_lossy()
            )),
            "permissions error"
        );

        // despite errors for some entries directories that is allowed should be created
        assert!(
            test_root.join("test3").exists(),
            "directory should be created"
        );
    });
}

#[test]
fn mkdir_verbose() {
    Playground::setup("mkdir_verbose", |dirs, _playground| {
        let test_root = dirs.test();

        let actual = nu!(cwd: test_root, "mkdir --verbose test1 test2");

        assert!(
            actual.err.contains(&format!(
                "Created dir {}",
                test_root.join("test1").to_string_lossy()
            )),
            "should show verbose info on creating directory"
        );

        assert!(
            actual.err.contains(&format!(
                "Created dir {}",
                test_root.join("test2").to_string_lossy()
            )),
            "should show verbose info on creating directory"
        );
    });
}
