use crate::playground::Playground;
use std::path::{Path, PathBuf};

<<<<<<< HEAD
use super::matchers::says;
use hamcrest2::assert_that;
use hamcrest2::prelude::*;

fn path(p: &Path) -> PathBuf {
    nu_path::canonicalize(p)
=======
fn path(p: &Path) -> PathBuf {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    nu_path::canonicalize_with(p, cwd)
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        .unwrap_or_else(|e| panic!("Couldn't canonicalize path {}: {:?}", p.display(), e))
}

#[test]
<<<<<<< HEAD
fn asserts_standard_out_expectation_from_nu_executable() {
    Playground::setup("topic", |_, nu| {
        assert_that!(nu.cococo("andres"), says().stdout("andres"));
    })
}

#[test]
fn asserts_standard_out_expectation_from_nu_executable_pipeline_fed() {
    Playground::setup("topic", |_, nu| {
        assert_that!(nu.pipeline("echo 'andres'"), says().stdout("andres"));
    })
}

#[test]
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
