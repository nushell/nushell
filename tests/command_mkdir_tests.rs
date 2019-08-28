mod helpers;

use h::Playground;
use helpers as h;
use std::path::{Path, PathBuf};

#[test]
fn creates_directory() {
    Playground::setup("mkdir_test_1", |dirs, _| {
        nu!(dirs.test(), "mkdir my_new_directory");

        let expected = dirs.test().join("my_new_directory");

        assert!(h::dir_exists_at(expected));
    })
}

#[test]
fn accepts_and_creates_directories() {
    Playground::setup("mkdir_test_2", |dirs, _| {
        nu!(dirs.test(), "mkdir dir_1 dir_2 dir_3");

        assert!(h::files_exist_at(
            vec![Path::new("dir_1"), Path::new("dir_2"), Path::new("dir_3")],
            dirs.test()
        ));
    })
}

#[test]
fn creates_intermediary_directories() {
    Playground::setup("mkdir_test_3", |dirs, _| {
        nu!(dirs.test(), "mkdir some_folder/another/deeper_one");

        let mut expected = PathBuf::from(dirs.test());
        expected.push("some_folder/another/deeper_one");

        assert!(h::dir_exists_at(expected));
    })
}
