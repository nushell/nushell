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
fn show_created_paths() {
    Playground::setup("mkdir_test_2", |dirs, _| {
        let actual = nu!(
         cwd: dirs.test(),
         pipeline(
             r#"
                 mkdir -s dir_1 dir_2 dir_3
                 | length
             "#
        ));

        assert!(files_exist_at(
            vec![Path::new("dir_1"), Path::new("dir_2"), Path::new("dir_3")],
            dirs.test()
        ));

        assert_eq!(actual.out, "3");
    })
}
