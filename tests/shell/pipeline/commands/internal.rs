use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::{pipeline, playground::Playground};

#[test]
fn takes_rows_of_nu_value_strings_and_pipes_it_to_stdin_of_external() {
    Playground::setup("internal_to_external_pipe_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
                Jonathan,1,New Zealand
                Andrés,1,Ecuador
                AndKitKatz,1,Estados Unidos
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            open nu_times.csv
            | get name
            | ^echo $it
            | chop
            | nth 3
            | echo $it
            "#
        ));

        assert_eq!(actual.out, "AndKitKat");
    })
}

#[test]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(
        cwd: ".",
        r#"echo "nushelll" | chop"#
    );

    assert_eq!(actual.out, "nushell");
}

mod parse {
    use nu_test_support::nu;

    /*
        The debug command's signature is:

        Usage:
        > debug {flags}

        flags:
        -h, --help: Display this help message
        -r, --raw: Prints the raw value representation.
    */

    #[test]
    fn errors_if_flag_passed_is_not_exact() {
        let actual = nu!(cwd: ".", "debug -ra");

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );

        let actual = nu!(cwd: ".", "debug --rawx");

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );
    }

    #[test]
    fn errors_if_flag_is_not_supported() {
        let actual = nu!(cwd: ".", "debug --ferris");

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );
    }

    #[test]
    fn errors_if_passed_an_unexpected_argument() {
        let actual = nu!(cwd: ".", "debug ferris");

        assert!(
            actual.err.contains("unexpected argument"),
            format!(
                "error message '{}' should contain 'unexpected argument'",
                actual.err
            )
        );
    }
}

mod tilde_expansion {
    use nu_test_support::fs::Stub::EmptyFile;
    use nu_test_support::playground::Playground;
    use nu_test_support::{nu, pipeline};

    #[test]
    #[should_panic]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
            echo ~
        "#
        );

        assert!(
            !actual.out.contains('~'),
            format!("'{}' should not contain ~", actual.out)
        );
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
                    echo "1~1"
                "#
        );

        assert_eq!(actual.out, "1~1");
    }

    #[test]
    fn proper_it_expansion() {
        Playground::setup("ls_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("andres.txt"),
                EmptyFile("gedge.txt"),
                EmptyFile("jonathan.txt"),
                EmptyFile("yehuda.txt"),
            ]);

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    ls | sort-by name | group-by type | each { get File.name | echo $it } | to json
                "#
            ));

            assert_eq!(
                actual.out,
                r#"["andres.txt","gedge.txt","jonathan.txt","yehuda.txt"]"#
            );
        })
    }
}
