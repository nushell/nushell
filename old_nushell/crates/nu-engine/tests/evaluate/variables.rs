use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::fs::{AbsolutePath, DisplayPath};
use nu_test_support::playground::{says, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn config_path_variable_present() {
    Playground::setup("nu_variable_test_1", |_, nu| {
        assert_that!(
            nu.pipeline("echo $nu.config-path"),
            says().stdout(nu.get_config())
        );
    })
}

#[test]
fn custom_config_path_variable_present() {
    Playground::setup("nu_variable_test_2", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            "skip_welcome_message = true",
        )]);

        assert_that!(
            nu.pipeline("echo $nu.config-path"),
            says().stdout(&file.display_path())
        );
    })
}

#[test]
fn scope_variable_with_alias_present() {
    Playground::setup("scope_variable_alias_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            "skip_welcome_message = true",
        )]);

        assert_that!(
            nu.pipeline("alias t = time; echo $scope.aliases | get t"),
            says().stdout("time")
        );
    })
}

#[test]
fn scope_variable_with_correct_number_of_aliases_present() {
    Playground::setup("scope_variable_alias_test_2", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            "skip_welcome_message = true",
        )]);

        assert_that!(
            nu.pipeline("alias v = version; alias t = time; echo $scope.aliases | length -c"),
            says().stdout("2")
        );
    })
}

#[test]
fn scope_variable_with_command_present() {
    Playground::setup("scope_commands_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            "skip_welcome_message = true",
        )]);

        assert_that!(
            nu.pipeline("def meaning-of-life [--number: int] { echo $number }; echo $scope.commands | get meaning-of-life"),
            says().stdout("--number")
        );
    })
}
