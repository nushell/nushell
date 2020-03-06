use nu_test_support::{nu, nu_error};

// #[test]
// fn shows_error_for_command_that_fails() {
//     let actual = nu_error!(
//         cwd: ".",
//         "fail"
//     );

//     assert!(actual.contains("External command failed"));
// }

#[test]
fn shows_error_for_command_not_found() {
    let actual = nu_error!(
        cwd: ".",
        "ferris_is_not_here.exe"
    );

    assert!(actual.contains("Command not found"));
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
                | cococo $it
                | lines
                | nth 1
                | echo $it
                "#
            ));

            assert_eq!(actual, "jonathan_likes_cake.txt");
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
                | chop $it
                | lines
                | nth 1
                | echo $it
                "#
            ));

            assert_eq!(actual, "AndrásWithKitKat");
        })
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
                    | cococo $it.nu_party_venue
                    | echo $it
                "#
            ));

            assert_eq!(actual, "zion");
        })
    }
}

mod stdin_evaluation {
    use super::{nu, nu_error};
    use nu_test_support::pipeline;

    #[test]
    fn does_not_panic_with_no_newline_in_stream() {
        let stderr = nu_error!(
            cwd: ".",
            pipeline(r#"
                nonu "where's the nuline?"
                | count
            "#
        ));

        assert_eq!(stderr, "");
    }

    #[test]
    fn does_not_block_indefinitely() {
        let stdout = nu!(
            cwd: ".",
            pipeline(r#"
                iecho yes
                | chop
                | chop
                | lines
                | first 1
            "#
        ));

        assert_eq!(stdout, "y");
    }
}

mod external_words {
    use super::nu;

    #[test]
    fn relaxed_external_words() {
        let actual = nu!(cwd: ".", r#"
        cococo joturner@foo.bar.baz
        "#);

        assert_eq!(actual, "joturner@foo.bar.baz");
    }
}

mod nu_commands {
    use super::nu;

    #[test]
    fn echo_internally_externally() {
        let actual = nu!(cwd: ".", r#"
        nu -c "echo 'foo'"
        "#);

        assert_eq!(actual, "foo");
    }
}

mod nu_script {
    use super::nu;

    #[test]
    fn run_nu_script() {
        let actual = nu!(cwd: "tests/fixtures/formats", r#"
        nu script.nu
        "#);

        assert_eq!(actual, "done");
    }

    #[test]
    fn run_nu_script_multiline() {
        let actual = nu!(cwd: "tests/fixtures/formats", r#"
        nu script_multiline.nu
        "#);

        assert_eq!(actual, "23");
    }
}

mod tilde_expansion {
    use super::nu;

    #[test]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
            cococo ~
        "#
        );

        assert!(
            !actual.contains('~'),
            format!("'{}' should not contain ~", actual)
        );
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
                    cococo "1~1"
                "#
        );

        assert_eq!(actual, "1~1");
    }
}
