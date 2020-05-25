use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::pipeline;
use nu_test_support::playground::Playground;

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
            | get origin
            | ^echo $it
            | nu --testbin chop
            | lines
            | nth 2
            | echo $it
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert_eq!(actual.out, "Ecuado");
    })
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

#[test]
fn argument_invocation() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo "foo" | echo $(echo $it)
            "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn invocation_handles_dot() {
    Playground::setup("invocation_handles_dot", |dirs, sandbox| {
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
            echo $(open nu_times.csv)
            | get name
            | nu --testbin chop $it
            | nth 3
            | echo $it
            "#
        ));

        assert_eq!(actual.out, "AndKitKat");
    })
}

#[test]
fn string_interpolation_with_it() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo "foo" | echo `{{$it}}`
            "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn string_interpolation_with_column() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "bob"}' | from json | echo `{{name}} is cool`
            "#
    );

    assert_eq!(actual.out, "bob is cool");
}

#[test]
fn string_interpolation_with_column2() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "fred"}' | from json | echo `also {{name}} is cool`
            "#
    );

    assert_eq!(actual.out, "also fred is cool");
}

#[test]
fn string_interpolation_with_column3() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "sally"}' | from json | echo `also {{name}}`
            "#
    );

    assert_eq!(actual.out, "also sally");
}

#[test]
fn string_interpolation_with_it_column_path() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "sammie"}' | from json | echo `{{$it.name}}`
        "#
    );

    assert_eq!(actual.out, "sammie");
}

#[test]
fn argument_invocation_reports_errors() {
    let actual = nu!(
        cwd: ".",
        "echo $(ferris_is_not_here.exe)"
    );

    assert!(actual.err.contains("Command not found"));
}

#[test]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(
        cwd: ".",
        r#"echo "nushelll" | nu --testbin chop"#
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
    use nu_test_support::nu;

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
}
