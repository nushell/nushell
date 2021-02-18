use super::support::Trusted;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

use serial_test::serial;

// Windows uses a different command to create an empty file
// so we need to have different content on windows.
const SCRIPTS: &str = if cfg!(target_os = "windows") {
    r#"[scripts]
        entryscripts = ["echo nul > hello.txt"]
        exitscripts = ["echo nul > bye.txt"]"#
} else {
    r#"[scripts]
        entryscripts = ["touch hello.txt"]
        exitscripts = ["touch bye.txt"]"#
};

#[test]
#[serial]
fn picks_up_env_keys_when_entering_trusted_directory() {
    Playground::setup("autoenv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let expected = "testvalue";

        let actual = Trusted::in_path(&dirs, || nu!(cwd: dirs.test(), "echo $nu.env.testkey"));

        assert_eq!(actual.out, expected);
    })
}

#[test]
#[serial]
fn picks_up_script_vars_when_entering_trusted_directory() {
    Playground::setup("autoenv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let expected = "myval";

        let actual = Trusted::in_path(&dirs, || nu!(cwd: dirs.test(), "echo $nu.env.myscript"));

        assert_eq!(actual.out, expected);
    })
}

#[test]
#[serial]
fn picks_up_env_keys_when_entering_trusted_directory_indirectly() {
    Playground::setup("autoenv_test_3", |dirs, sandbox| {
        sandbox.mkdir("crates");
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            r#"[env]
                nu-version = "0.27.1" "#,
        )]);

        let expected = "0.27.1";

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test().join("crates"), r#"
                cd ../../autoenv_test_3
                echo $nu.env.nu-version
            "#)
        });

        assert_eq!(actual.out, expected);
    })
}

#[test]
#[serial]
fn entering_a_trusted_directory_runs_entry_scripts() {
    Playground::setup("autoenv_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), pipeline(r#"
                ls
                | where name == "hello.txt"
                | get name
            "#))
        });

        assert_eq!(actual.out, "hello.txt");
    })
}

#[test]
#[serial]
fn leaving_a_trusted_directory_runs_exit_scripts() {
    Playground::setup("autoenv_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
              cd ..
              ls autoenv_test_5 | get name | path basename | where $it == "bye.txt"
            "#)
        });

        assert_eq!(actual.out, "bye.txt");
    })
}

#[test]
#[serial]
fn entry_scripts_are_called_when_revisiting_a_trusted_directory() {
    Playground::setup("autoenv_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                do { rm hello.txt ; = $nothing } ; # Silence file deletion message from output
                cd ..
                cd autoenv_test_6
                ls | where name == "hello.txt" | get name
            "#)
        });

        assert_eq!(actual.out, "hello.txt");
    })
}

#[test]
#[serial]
fn given_a_trusted_directory_with_entry_scripts_when_entering_a_subdirectory_entry_scripts_are_not_called(
) {
    Playground::setup("autoenv_test_7", |dirs, sandbox| {
        sandbox.mkdir("time_to_cook_arepas");
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                cd time_to_cook_arepas
                ls | where name == "hello.txt" | count
            "#)
        });

        assert_eq!(actual.out, "0");
    })
}

#[test]
#[serial]
fn given_a_trusted_directory_with_exit_scripts_when_entering_a_subdirectory_exit_scripts_are_not_called(
) {
    Playground::setup("autoenv_test_8", |dirs, sandbox| {
        sandbox.mkdir("time_to_cook_arepas");
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                r#"[env]
                testkey = "testvalue"
        
               [scriptvars]
                myscript = "echo myval"
            "#,
                SCRIPTS
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                cd time_to_cook_arepas
                ls | where name == "bye.txt" | count
            "#)
        });

        assert_eq!(actual.out, "0");
    })
}

#[test]
#[serial]
fn given_a_hierachy_of_trusted_directories_when_entering_in_any_nested_ones_should_carry_over_variables_set_from_the_root(
) {
    Playground::setup("autoenv_test_9", |dirs, sandbox| {
        sandbox.mkdir("nu_plugin_rb");
        sandbox.with_files(vec![
            FileWithContent(
                ".nu-env",
                r#"[env]
                organization = "nushell""#,
            ),
            FileWithContent(
                "nu_plugin_rb/.nu-env",
                r#"[env]
                language = "Ruby""#,
            ),
        ]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test().parent().unwrap(), r#"
                do { autoenv trust autoenv_test_9/nu_plugin_rb ; = $nothing } # Silence autoenv trust message from output
                cd autoenv_test_9/nu_plugin_rb
                echo $nu.env.organization
            "#)
        });

        assert_eq!(actual.out, "nushell");
    })
}

#[test]
#[serial]
fn given_a_hierachy_of_trusted_directories_nested_ones_should_overwrite_variables_from_parent_directories(
) {
    Playground::setup("autoenv_test_10", |dirs, sandbox| {
        sandbox.mkdir("nu_plugin_rb");
        sandbox.with_files(vec![
            FileWithContent(
                ".nu-env",
                r#"[env]
                organization = "nushell""#,
            ),
            FileWithContent(
                "nu_plugin_rb/.nu-env",
                r#"[env]
                organization = "Andrab""#,
            ),
        ]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test().parent().unwrap(), r#"
                do { autoenv trust autoenv_test_10/nu_plugin_rb ; = $nothing } # Silence autoenv trust message from output
                cd autoenv_test_10/nu_plugin_rb
                echo $nu.env.organization
            "#)
        });

        assert_eq!(actual.out, "Andrab");
    })
}

#[test]
#[serial]
fn given_a_hierachy_of_trusted_directories_going_back_restores_overwritten_variables() {
    Playground::setup("autoenv_test_11", |dirs, sandbox| {
        sandbox.mkdir("nu_plugin_rb");
        sandbox.with_files(vec![
            FileWithContent(
                ".nu-env",
                r#"[env]
                organization = "nushell""#,
            ),
            FileWithContent(
                "nu_plugin_rb/.nu-env",
                r#"[env]
                organization = "Andrab""#,
            ),
        ]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test().parent().unwrap(), r#"
                do { autoenv trust autoenv_test_11/nu_plugin_rb ; = $nothing } # Silence autoenv trust message from output
                cd autoenv_test_11
                cd nu_plugin_rb
                do { rm ../.nu-env ; = $nothing } # By deleting the root nu-env we have guarantees that the variable gets restored (not by autoenv when re-entering)
                cd ..
                echo $nu.env.organization
            "#)
        });

        assert_eq!(actual.out, "nushell");
    })
}
