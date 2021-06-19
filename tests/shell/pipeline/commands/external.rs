use nu_test_support::nu;

#[cfg(feature = "which")]
#[test]
fn shows_error_for_command_not_found() {
    let actual = nu!(
        cwd: ".",
        "ferris_is_not_here.exe"
    );

    assert!(actual.err.contains("Command not found"));
}

#[cfg(feature = "which")]
#[test]
fn shows_error_for_command_not_found_in_pipeline() {
    let actual = nu!(
        cwd: ".",
        "ferris_is_not_here.exe | echo done"
    );

    assert!(actual.err.contains("Command not found"));
}

#[cfg(feature = "which")]
#[test]
fn automatically_change_directory() {
    use nu_test_support::playground::Playground;

    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("autodir");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                autodir
                echo (pwd)
            "#
        );

        assert!(actual.out.ends_with("autodir"));
    })
}

#[test]
fn automatically_change_directory_with_trailing_slash_and_same_name_as_command() {
    use nu_test_support::playground::Playground;

    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("cd");

        let actual = nu!(
            cwd: dirs.test(),
            r#"
                cd/
                pwd
            "#
        );

        assert!(actual.out.ends_with("cd"));
    })
}

#[test]
fn correctly_escape_external_arguments() {
    let actual = nu!(cwd: ".", r#"^echo '$0'"#);

    assert_eq!(actual.out, "$0");
}

#[test]
fn execute_binary_in_string() {
    let actual = nu!(
    cwd: ".",
    r#"
        let cmd = echo
        ^$"($cmd)" '$0'
    "#);

    assert_eq!(actual.out, "$0");
}

#[test]
fn redirects_custom_command_external() {
    let actual = nu!(cwd: ".", r#"def foo [] { nu --testbin cococo foo bar }; foo | str length "#);

    assert_eq!(actual.out, "8");
}

mod it_evaluation {
    use super::nu;
    use nu_test_support::fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed};
    use nu_test_support::{pipeline, playground::Playground};

    #[test]
    fn takes_rows_of_nu_value_strings() {
        Playground::setup("it_argument_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("jonathan_likes_cake.txt"),
                EmptyFile("andres_likes_arepas.txt"),
            ]);

            let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | sort-by name
                | get name
                | each { nu --testbin cococo $it | lines }
                | nth 1
                "#
            ));

            assert_eq!(actual.out, "jonathan_likes_cake.txt");
        })
    }

    #[test]
    fn takes_rows_of_nu_value_lines() {
        Playground::setup("it_argument_test_2", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContentToBeTrimmed(
                "nu_candies.txt",
                r#"
                    AndrásWithKitKatzz
                    AndrásWithKitKatz
                "#,
            )]);

            let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nu_candies.txt
                | lines
                | each { nu --testbin chop $it | lines}
                | nth 1
                "#
            ));

            assert_eq!(actual.out, "AndrásWithKitKat");
        })
    }

    #[test]
    fn can_properly_buffer_lines_externally() {
        let actual = nu!(
            cwd: ".",
            r#"
                nu --testbin repeater c 8197 | lines | length
            "#
        );

        assert_eq!(actual.out, "1");
    }
    #[test]
    fn supports_fetching_given_a_column_path_to_it() {
        Playground::setup("it_argument_test_3", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "sample.toml",
                r#"
                    nu_party_venue = "zion"
                "#,
            )]);

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    open sample.toml
                    | each { nu --testbin cococo $it.nu_party_venue }
                "#
            ));

            assert_eq!(actual.out, "zion");
        })
    }
}

mod stdin_evaluation {
    use super::nu;
    use nu_test_support::pipeline;

    #[test]
    fn does_not_panic_with_no_newline_in_stream() {
        let actual = nu!(
            cwd: ".",
            pipeline(r#"
                nu --testbin nonu "where's the nuline?"
                | length
            "#
        ));

        assert_eq!(actual.err, "");
    }

    #[test]
    fn does_not_block_indefinitely() {
        let stdout = nu!(
            cwd: ".",
            pipeline(r#"
                nu --testbin iecho yes
                | nu --testbin chop
                | nu --testbin chop
                | lines
                | first 1
            "#
        ))
        .out;

        assert_eq!(stdout, "y");
    }
}

mod external_words {
    use super::nu;
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::{pipeline, playground::Playground};
    #[test]
    fn relaxed_external_words() {
        let actual = nu!(cwd: ".", r#"
        nu --testbin cococo joturner@foo.bar.baz
        "#);

        assert_eq!(actual.out, "joturner@foo.bar.baz");
    }

    #[test]
    fn no_escaping_for_single_quoted_strings() {
        let actual = nu!(cwd: ".", r#"
        nu --testbin cococo 'test "things"'
        "#);

        assert_eq!(actual.out, "test \"things\"");
    }

    #[test]
    fn external_arg_with_quotes() {
        Playground::setup("external_arg_with_quotes", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "sample.toml",
                r#"
                    nu_party_venue = "zion"
                "#,
            )]);

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    nu --testbin meow "sample.toml" | from toml | get nu_party_venue
                "#
            ));

            assert_eq!(actual.out, "zion");
        })
    }
}

mod nu_commands {
    use super::nu;

    #[test]
    fn echo_internally_externally() {
        let actual = nu!(cwd: ".", r#"
        nu -c "echo 'foo'"
        "#);

        assert_eq!(actual.out, "foo");
    }
}

mod nu_script {
    use super::nu;

    #[test]
    fn run_nu_script() {
        let actual = nu!(cwd: "tests/fixtures/formats", r#"
        nu script.nu
        "#);

        assert_eq!(actual.out, "done");
    }

    #[test]
    fn run_nu_script_multiline() {
        let actual = nu!(cwd: "tests/fixtures/formats", r#"
        nu script_multiline.nu
        "#);

        assert_eq!(actual.out, "23");
    }
}

mod tilde_expansion {
    use super::nu;

    #[test]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
            nu --testbin cococo  ~
        "#
        );

        assert!(!actual.out.contains('~'));
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
                nu --testbin cococo  "1~1"
                "#
        );

        assert_eq!(actual.out, "1~1");
    }
}

mod external_command_arguments {
    use super::nu;
    use nu_test_support::fs::Stub::EmptyFile;
    use nu_test_support::{pipeline, playground::Playground};
    #[test]
    fn expands_table_of_primitives_to_positional_arguments() {
        Playground::setup(
            "expands_table_of_primitives_to_positional_arguments",
            |dirs, sandbox| {
                sandbox.with_files(vec![
                    EmptyFile("jonathan_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    nu --testbin cococo (ls | get name)
                "#
                ));

                assert_eq!(
                    actual.out,
                    "andres_likes_arepas.txt ferris_not_here.txt jonathan_likes_cake.txt"
                );
            },
        )
    }

    #[test]
    fn proper_subexpression_paths_in_external_args() {
        Playground::setup(
            "expands_table_of_primitives_to_positional_arguments",
            |dirs, sandbox| {
                sandbox.with_files(vec![
                    EmptyFile("jonathan_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    nu --testbin cococo (ls | sort-by name | get name).1
                "#
                ));

                assert_eq!(actual.out, "ferris_not_here.txt");
            },
        )
    }
}
