mod pipeline {
    use nu_test_support::fs::Stub::EmptyFile;
    use nu_test_support::playground::Playground;
    use nu_test_support::{nu, nu_error, pipeline};

    #[test]
    fn can_process_row_as_it_argument_to_an_external_command_given_the_it_data_is_a_string() {
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
                    | ^echo $it
                "#
            ));

            #[cfg(windows)]
            assert_eq!(actual, "andres_likes_arepas.txt jonathan_likes_cake.txt");

            #[cfg(not(windows))]
            assert_eq!(actual, "andres_likes_arepas.txtjonathan_likes_cake.txt");
        })
    }

    #[test]
    fn doesnt_break_on_utf8_command() {
        let actual = nu!(
            cwd: ".", pipeline(
            r#"
                echo ö
            "#
        ));

        assert!(
            actual.contains("ö"),
            format!("'{}' should contain ö", actual)
        );
    }

    #[test]
    fn can_process_row_as_it_argument_to_an_external_command_given_the_it_data_is_one_string_line()
    {
        Playground::setup("it_argument_test_2", |dirs, sandbox| {
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
                    | lines
                    | ^echo $it
                "#
            ));

            #[cfg(windows)]
            assert_eq!(actual, "andres_likes_arepas.txt jonathan_likes_cake.txt");

            #[cfg(not(windows))]
            assert_eq!(actual, "andres_likes_arepas.txtjonathan_likes_cake.txt");
        })
    }

    #[test]
    fn can_process_stdout_of_external_piped_to_stdin_of_external() {
        let actual = nu!(
            cwd: "tests/fixtures",
            "^echo 1 | ^cat"
        );

        assert!(actual.contains("1"));
    }

    #[test]
    fn can_process_row_from_internal_piped_to_stdin_of_external() {
        let actual = nu!(
            cwd: "tests/fixtures",
            "echo \"1\" | ^cat"
        );

        assert!(actual.contains("1"));
    }

    #[test]
    fn shows_error_for_external_command_that_fails() {
        let actual = nu_error!(
            cwd: "tests/fixtures",
            "^false"
        );

        assert!(actual.contains("External command failed"));
    }

    mod expands_tilde {
        use super::nu;

        #[test]
        fn as_home_directory_when_passed_as_argument_and_begins_with_tilde_to_an_external() {
            let actual = nu!(
                cwd: std::path::PathBuf::from("."),
                r#"
                    sh -c "echo ~"
                "#
            );

            assert!(
                !actual.contains('~'),
                format!("'{}' should not contain ~", actual)
            );
        }

        #[test]
        fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde_to_an_external() {
            let actual = nu!(
                cwd: std::path::PathBuf::from("."),
                r#"
                    sh -c "echo 1~1"
                "#
            );

            assert_eq!(actual, "1~1");
        }
    }
}
