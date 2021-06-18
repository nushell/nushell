use nu_test_support::fs::AbsolutePath;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::{says, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn clearing_config_clears_config() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            nu.pipeline("config clear | ignore; config get skip_welcome_message"),
            says().stdout("")
        );
        let config_contents = std::fs::read_to_string(file).expect("Could not read file");
        assert!(config_contents.is_empty());
    });
}

#[test]
fn config_get_returns_value() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            //Clears config
            nu.pipeline("config get skip_welcome_message"),
            says().stdout("true")
        );
    });
}

#[test]
fn config_set_sets_value() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            //Clears config
            nu.pipeline("config set key value | ignore; config get key"),
            says().stdout("value")
        );
        let config_contents = std::fs::read_to_string(file).expect("Could not read file");
        assert!(config_contents.contains("key = \"value\""));
    });
}

#[test]
fn config_set_into_sets_value() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            //Clears config
            nu.pipeline("echo value | config set_into key | ignore; config get key"),
            says().stdout("value")
        );
        let config_contents = std::fs::read_to_string(file).expect("Could not read file");
        assert!(config_contents.contains("key = \"value\""));
    });
}

#[test]
fn config_rm_removes_value() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                key = "value"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            nu.pipeline("config remove key | ignore; config get key"),
            says().stdout("")
        );
        let config_contents = std::fs::read_to_string(file).expect("Could not read file");
        assert!(!config_contents.contains("key = \"value\""));
    });
}

#[test]
fn config_path_returns_correct_path() {
    Playground::setup("environment_syncing_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
                skip_welcome_message = true
            "#,
        )]);

        assert_that!(
            nu.pipeline("config path"),
            says().stdout(&file.inner.to_string_lossy().to_string())
        );
    });
}
