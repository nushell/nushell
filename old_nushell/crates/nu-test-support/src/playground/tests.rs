use crate::playground::Playground;
use std::path::{Path, PathBuf};

use super::matchers::says;
use hamcrest2::assert_that;
use hamcrest2::prelude::*;

fn path(p: &Path) -> PathBuf {
    nu_path::canonicalize(p)
        .unwrap_or_else(|e| panic!("Couldn't canonicalize path {}: {:?}", p.display(), e))
}

#[test]
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
