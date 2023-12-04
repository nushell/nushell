use crate::playground::Playground;
use std::path::{Path, PathBuf};

fn path(p: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    nu_path::canonicalize_with(p, cwd)
        .unwrap_or_else(|e| panic!("Couldn't canonicalize path {}: {:?}", p.display(), e))
}

#[test]
fn current_working_directory_in_sandbox_directory_created() {
    Playground::setup("topic", |dirs, nu| {
        let original_cwd = dirs.test();
        nu.within("some_directory_within");

        assert_eq!(path(nu.cwd()), original_cwd.join("some_directory_within"));
    })
}

#[test]
fn current_working_directory_back_to_root_from_anywhere() {
    Playground::setup("topic", |dirs, nu| {
        let original_cwd = dirs.test();

        nu.within("some_directory_within");
        nu.back_to_playground();

        assert_eq!(path(nu.cwd()), *original_cwd);
    })
}
