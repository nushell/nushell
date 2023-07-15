use std::fs::create_dir;

use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn create() {
    Playground::setup("directory_create", |dirs, _| {
        let _ = nu!(cwd: dirs.test(), "mkdir test_directory");
        let dir_path = dirs.test().join("test_directory");
        assert!(dir_path.metadata().map(|x| x.is_dir()).unwrap_or_default());
    })
}

#[test]
fn remove() {
    Playground::setup("directory_remove", |dirs, _| {
        let dir_path = dirs.test().join("test_directory");
        create_dir(dir_path.clone()).expect("cant make test dir");
        let _ = nu!(cwd: dirs.test(), "rm test_directory");
        assert!(!dir_path.exists());
    })
}
