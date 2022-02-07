use super::support::Trusted;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

use serial_test::serial;

const SCRIPTS: &str = r#"startup = ["touch hello.txt"]
    on_exit = ["touch bye.txt"]"#;

#[test]
#[serial]
fn picks_up_env_keys_when_entering_trusted_directory() {
    Playground::setup("autoenv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
            ),
        )]);

        let expected = "testvalue";

        let actual = Trusted::in_path(&dirs, || nu!(cwd: dirs.test(), "echo $env.testkey"));

        assert_eq!(actual.out, expected);
    })
}

#[cfg(feature = "which-support")]
#[test]
#[serial]
fn picks_up_and_lets_go_env_keys_when_entering_trusted_directory_with_implied_cd() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");
        sandbox.with_files(vec![
            FileWithContent(
                "foo/.nu-env",
                r#"[env]
               testkey = "testvalue"
                "#,
            ),
            FileWithContent(
                "foo/bar/.nu-env",
                r#"
                [env]
               bar = "true"
                "#,
            ),
        ]);
        let actual = nu!(
            cwd: dirs.test(),
            r#"
            do {autoenv trust -q foo ; = $nothing }
            foo
            echo $env.testkey"#
        );
        assert_eq!(actual.out, "testvalue");
        //Assert testkey is gone when leaving foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"
            do {autoenv trust -q foo; = $nothing } ;
            foo
            ..
            echo $env.testkey
            "#
        );
        assert!(actual.err.contains("Unknown"));
        //Assert testkey is present also when jumping over foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"
            do {autoenv trust -q foo; = $nothing } ;
            do {autoenv trust -q foo/bar; = $nothing } ;
            foo/bar
            echo $env.testkey
            echo $env.bar
            "#
        );
        assert_eq!(actual.out, "testvaluetrue");
        //Assert bar removed after leaving bar
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo;
               foo/bar
               ../..
               echo $env.bar"#
        );
        assert!(actual.err.contains("Unknown"));
    });
}

#[test]
#[serial]
#[ignore]
fn picks_up_script_vars_when_entering_trusted_directory() {
    Playground::setup("autoenv_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            &format!(
                "{}\n{}",
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
            ),
        )]);

        let expected = "myval";

        let actual = Trusted::in_path(&dirs, || nu!(cwd: dirs.test(), "echo $env.myscript"));

        // scriptvars are not supported
        // and why is myval expected when myscript is "echo myval"
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
                nu-ver = "0.30.0" "#,
        )]);

        let expected = "0.30.0";

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test().join("crates"), r#"
                cd ../../autoenv_test_3
                echo $env.nu-ver
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
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"
            "#
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
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
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
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                do { rm hello.txt | ignore } ; # Silence file deletion message from output
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
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                cd time_to_cook_arepas
                ls | where name == "hello.txt" | length
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
                SCRIPTS,
                r#"[env]
                testkey = "testvalue"

               [scriptvars]
                myscript = "echo myval"
            "#
            ),
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                cd time_to_cook_arepas
                ls | where name == "bye.txt" | length
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
                do { autoenv trust -q autoenv_test_9/nu_plugin_rb ; = $nothing } # Silence autoenv trust -q message from output
                cd autoenv_test_9/nu_plugin_rb
                echo $env.organization
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
                do { autoenv trust -q autoenv_test_10/nu_plugin_rb ; = $nothing } # Silence autoenv trust -q message from output
                cd autoenv_test_10/nu_plugin_rb
                echo $env.organization
            "#)
        });

        assert_eq!(actual.out, "Andrab");
    })
}

#[test]
#[serial]
#[cfg(not(windows))] //TODO figure out why this test doesn't work on windows
fn local_config_should_not_be_added_when_running_scripts() {
    Playground::setup("autoenv_test_10", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.with_files(vec![
            FileWithContent(
                ".nu-env",
                r#"[env]
                organization = "nu""#,
            ),
            FileWithContent(
                "foo/.nu-env",
                r#"[env]
                organization = "foo""#,
            ),
            FileWithContent(
                "script.nu",
                r#"cd foo
                echo $env.organization"#,
            ),
        ]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                do { autoenv trust -q foo } # Silence autoenv trust message from output
                nu script.nu
            "#)
        });

        assert_eq!(actual.out, "nu");
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
                do { autoenv trust -q autoenv_test_11/nu_plugin_rb } # Silence autoenv trust message from output
                cd autoenv_test_11
                cd nu_plugin_rb
                do { rm ../.nu-env | ignore } # By deleting the root nu-env we have guarantees that the variable gets restored (not by autoenv when re-entering)
                cd ..
                echo $env.organization
            "#)
        });

        assert_eq!(actual.out, "nushell");
    })
}

#[cfg(feature = "which-support")]
#[test]
#[serial]
fn local_config_env_var_present_and_removed_correctly() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");
        sandbox.with_files(vec![FileWithContent(
            "foo/.nu-env",
            r#"[env]
               testkey = "testvalue"
                "#,
        )]);
        //Assert testkey is not present before entering directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo;
               echo $env.testkey"#
        );
        assert!(actual.err.contains("Unknown"));
        //Assert testkey is present in foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; cd foo
               echo $env.testkey"#
        );
        assert_eq!(actual.out, "testvalue");
        //Assert testkey is present also in subdirectories
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; cd foo
               cd bar
               echo $env.testkey"#
        );
        assert_eq!(actual.out, "testvalue");
        //Assert testkey is present also when jumping over foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; cd foo/bar
               echo $env.testkey"#
        );
        assert_eq!(actual.out, "testvalue");
        //Assert testkey removed after leaving foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; cd foo
               cd ..
               echo $env.testkey"#
        );
        assert!(actual.err.contains("Unknown"));
    });
}

#[cfg(feature = "which-support")]
#[test]
#[serial]
fn local_config_env_var_gets_overwritten() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.mkdir("foo/bar");
        sandbox.with_files(vec![
            FileWithContent(
                "foo/.nu-env",
                r#"[env]
                overwrite_me = "foo"
                "#,
            ),
            FileWithContent(
                "foo/bar/.nu-env",
                r#"[env]
                overwrite_me = "bar"
                "#,
            ),
        ]);
        //Assert overwrite_me is not present before entering directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo;
               echo $env.overwrite_me"#
        );
        assert!(actual.err.contains("Unknown"));
        //Assert overwrite_me is foo in foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; cd foo
               echo $env.overwrite_me"#
        );
        assert_eq!(actual.out, "foo");
        //Assert overwrite_me is bar in bar
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               autoenv trust -q foo/bar
               cd foo
               cd bar
               echo $env.overwrite_me"#
        );
        assert_eq!(actual.out, "bar");
        //Assert overwrite_me is present also when jumping over foo
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; autoenv trust -q foo/bar; cd foo/bar
               echo $env.overwrite_me
            "#
        );
        assert_eq!(actual.out, "bar");
        //Assert overwrite_me removed after leaving bar
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo; autoenv trust -q foo/bar; cd foo
               cd bar
               cd ..
               echo $env.overwrite_me"#
        );
        assert_eq!(actual.out, "foo");
    });
}

#[cfg(feature = "which-support")]
#[test]
#[serial]
fn autoenv_test_entry_scripts() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo/bar");

        // Windows uses a different command to create an empty file so we need to have different content on windows.
        let nu_env = if cfg!(target_os = "windows") {
            r#"startup = ["echo nul > hello.txt"]"#
        } else {
            r#"startup = ["touch hello.txt"]"#
        };

        sandbox.with_files(vec![FileWithContent("foo/.nu-env", nu_env)]);

        // Make sure entryscript is run when entering directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo
               ls | where name == "hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        // Make sure entry scripts are also run when jumping over directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo/bar
               ls .. | where name == "../hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        // Entryscripts should not run after changing to a subdirectory.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo
               rm hello.txt
               cd bar
               ls .. | where name == "../hello.txt" | length"#
        );
        assert!(actual.out.contains('0'));
    });
}

#[cfg(feature = "which-support")]
#[test]
#[serial]
fn autoenv_test_exit_scripts() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo/bar");

        // Windows uses a different command to create an empty file so we need to have different content on windows.
        let nu_env = r#"on_exit = ["touch bye.txt"]"#;

        sandbox.with_files(vec![FileWithContent("foo/.nu-env", nu_env)]);

        // Make sure exitscript is run
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo
               cd ..
               ls foo | where name =~ "bye.txt" | length
               rm foo/bye.txt | ignore; cd .
               "#
        );
        assert_eq!(actual.out, "1");

        // Entering a subdir should not trigger exitscripts
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo
               cd bar
               ls .. | where name =~ "bye.txt" | length"#
        );
        assert_eq!(actual.out, "0");

        // Also run exitscripts when jumping over directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust -q foo
               cd foo/bar
               cd ../..
               ls foo | where name =~ "bye.txt" | length
               rm foo/bye.txt | ignore; cd ."#
        );
        assert_eq!(actual.out, "1");
    });
}

#[test]
#[serial]
#[cfg(unix)]
fn prepends_path_from_local_config() {
    //If this test fails for you, make sure that your environment from which you start nu
    //contains some env vars
    Playground::setup("autoenv_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            r#"
            path = ["/hi", "/nushell"]
            "#,
        )]);

        let expected = "[\"/hi\",\"/nushell\",";

        let actual = Trusted::in_path(&dirs, || nu!(cwd: dirs.test(), "echo $nu.path | to json"));
        // assert_eq!("", actual.out);
        assert!(actual.out.starts_with(expected));
        assert!(actual.out.len() > expected.len());
    })
}
