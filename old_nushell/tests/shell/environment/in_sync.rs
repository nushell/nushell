use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::fs::{AbsolutePath, DisplayPath};
use nu_test_support::playground::{says, Playground};

use std::path::PathBuf;

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn setting_environment_value_to_configuration_should_pick_up_into_in_memory_environment_on_runtime()
{
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true

                [env]
                SHELL = "/local/nu"
                "#,
        )]);

        assert_that!(
            nu.pipeline("config set env.USER NUNO | ignore")
                .and_then("echo $nu.env.USER"),
            says().stdout("NUNO")
        );
    });
}

#[test]
fn inherited_environment_values_not_present_in_configuration_should_pick_up_into_in_memory_environment(
) {
    Playground::setup("environment_syncing_test_2", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true

                [env]
                SHELL = "/local/nu"
                "#,
        )])
        .with_config(&file)
        .with_env("USER", "NUNO");

        assert_that!(nu.pipeline("echo $nu.env.USER"), says().stdout("NUNO"));
    });
}

#[test]
fn environment_values_present_in_configuration_overwrites_inherited_environment_values() {
    Playground::setup("environment_syncing_test_3", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true

                [env]
                SHELL = "/usr/bin/you_already_made_the_nu_choice"
                "#,
        )])
        .with_config(&file)
        .with_env("SHELL", "/usr/bin/sh");

        assert_that!(
            nu.pipeline("echo $nu.env.SHELL"),
            says().stdout("/usr/bin/you_already_made_the_nu_choice")
        );
    });
}

#[test]
fn inherited_environment_path_values_not_present_in_configuration_should_pick_up_into_in_memory_environment(
) {
    Playground::setup("environment_syncing_test_4", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        let expected_paths = vec![
            PathBuf::from("/Users/andresrobalino/.volta/bin"),
            PathBuf::from("/Users/mosqueteros/bin"),
            PathBuf::from("/path/to/be/added"),
        ]
        .iter()
        .map(|p| p.display_path())
        .collect::<Vec<_>>()
        .join("-");

        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true

                path = ["/Users/andresrobalino/.volta/bin", "/Users/mosqueteros/bin"]
                "#,
        )])
        .with_config(&file)
        .with_env(
            nu_test_support::NATIVE_PATH_ENV_VAR,
            &PathBuf::from("/path/to/be/added").display_path(),
        );

        assert_that!(
            nu.pipeline("echo $nu.path | str collect '-'"),
            says().stdout(&expected_paths)
        );
    });
}
