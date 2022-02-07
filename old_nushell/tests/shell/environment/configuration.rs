use nu_test_support::fs::{file_contents, Stub::FileWithContent};
use nu_test_support::fs::{AbsolutePath, DisplayPath};
use nu_test_support::pipeline as input;
use nu_test_support::playground::{says, Executable, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn clears_the_configuration() {
    Playground::setup("config_clear_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
            skip_welcome_message = true
            pivot_mode = "arepas"
            "#,
        )]);

        assert!(nu.pipeline("config clear").execute().is_ok());
        assert!(file_contents(&file).is_empty());
    });
}

#[test]
fn retrieves_config_values() {
    Playground::setup("config_get_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
            skip_welcome_message = true

            [arepa]
            colors = ["yellow", "white"]
            "#,
        )]);

        assert_that!(
            nu.pipeline("config get arepa.colors.0"),
            says().stdout("yellow")
        );
    })
}

#[test]
fn sets_a_config_value() {
    Playground::setup("config_set_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
            skip_welcome_message = true
            
            [nu]
            meal = "taco"
            "#,
        )]);

        assert!(nu.pipeline("config set nu.meal 'arepa'").execute().is_ok());

        assert_that!(nu.pipeline("config get nu.meal"), says().stdout("arepa"));
    })
}

#[test]
fn sets_config_values_into_one_property() {
    Playground::setup("config_set_into_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
            skip_welcome_message = true
            "#,
        )]);

        assert!(nu
            .pipeline(&input(
                r#"
            echo ["amarillo", "blanco"]
            | config set_into arepa_colors
        "#,
            ))
            .execute()
            .is_ok());

        assert_that!(
            nu.pipeline("config get arepa_colors.1"),
            says().stdout("blanco")
        );
    })
}

#[test]
fn config_path() {
    Playground::setup("config_path_test", |dirs, nu| {
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
            says().stdout(&file.display_path())
        );
    })
}

#[test]
fn removes_config_values() {
    Playground::setup("config_remove_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![FileWithContent(
            "config.toml",
            r#"
            skip_welcome_message = true
            "#,
        )]);

        assert!(nu
            .pipeline("config remove skip_welcome_message")
            .execute()
            .is_ok());
        assert!(file_contents(&file).is_empty());
    })
}
