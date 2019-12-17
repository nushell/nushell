use nu_test_support::fs::files_exist_at;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
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
