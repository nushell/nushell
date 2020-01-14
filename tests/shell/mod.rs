mod pipeline {
    use nu_test_support::{nu, nu_error};

    #[test]
    fn doesnt_break_on_utf8() {
        let actual = nu!(cwd: ".", "echo ö");

        assert_eq!(actual, "ö", "'{}' should contain ö", actual);
    }

    #[test]
    fn can_process_stdout_of_external_piped_to_stdin_of_external() {
        let actual = nu!(
            cwd: ".",
            r#"cococo "nushelll" | chop"#
        );

        assert_eq!(actual, "nushell");
    }

    #[test]
    fn can_process_one_row_from_internal_piped_to_stdin_of_external() {
        let actual = nu!(
            cwd: ".",
            r#"echo "nushelll" | chop"#
        );

        assert_eq!(actual, "nushell");
    }

    #[test]
    fn shows_error_for_external_command_that_fails() {
        let actual = nu_error!(
            cwd: ".",
            "fail"
        );

        assert!(actual.contains("External command failed"));
    }

    mod expands_tilde {
        use super::nu;

        #[test]
        fn as_home_directory_when_passed_as_argument_and_begins_with_tilde_to_an_external() {
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
        fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde_to_an_external() {
            let actual = nu!(
                cwd: ".",
                r#"
                    cococo "1~1"
                "#
            );

            assert_eq!(actual, "1~1");
        }
    }
}
