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
            says().to_stdout("yellow")
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

        assert_that!(nu.pipeline("config get nu.meal"), says().to_stdout("arepa"));
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
            says().to_stdout("blanco")
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
            says().to_stdout(&file.display_path())
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

#[test]
fn runs_configuration_startup_commands() {
    Playground::setup("init_config_startup_commands_test", |dirs, nu| {
        let file = AbsolutePath::new(dirs.config_fixtures().join("startup.toml"));

        nu.with_config(&file);

        assert_that!(nu.pipeline("hello-world"), says().to_stdout("Nu World"));
    });
}

#[test]
fn runs_configuration_startup_commands_hard() {
    Playground::setup("config_startup_is_sourced", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("config.toml"));

        nu.with_config(&file);
        nu.with_files(vec![
            //Normal def is sourced, source is run and sources file
            FileWithContent(
                "config.toml",
                r#"
                skip_welcome_message = true
                startup = ["def def_in_startup [] { echo 'get ready' }", "source defs.nu"]
                "#,
            ),
            FileWithContent(
                "defs.nu",
                r#"
                def def_in_script [] { echo "we are going to the moon" }
            "#,
            ),
        ]);

        assert_that!(
            nu.pipeline("def_in_startup").and_then("def_in_script"),
            says().to_stdout("get readywe are going to the moon")
        );
    })
}

#[test]
fn runs_configuration_startup_commands_in_current_dir() {
    Playground::setup("config_startup_is_sourced", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("foo/config.toml"));
        nu.within("foo");
        nu.with_config(&file);
        nu.with_files(vec![
            //touch is executed in current dir
            FileWithContent(
                "config.toml",
                r#"
                skip_welcome_message = true
                startup = ["touch bar"]
                "#,
            ),
        ]);

        assert_that!(nu.pipeline("echo hi"), says().to_stdout("hi"));
        assert!(dirs.test().join("foo/bar").exists());
    })
}
