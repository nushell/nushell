use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;

#[cfg(feature = "which-support")]
#[test]
fn shows_error_for_command_not_found() {
    let actual = nu!("ferris_is_not_here.exe");

    assert!(!actual.err.is_empty());
}

#[cfg(feature = "which-support")]
#[test]
fn shows_error_for_command_not_found_in_pipeline() {
    let actual = nu!("ferris_is_not_here.exe | echo done");

    assert!(!actual.err.is_empty());
}

#[ignore] // jt: we can't test this using the -c workaround currently
#[cfg(feature = "which-support")]
#[test]
fn automatically_change_directory() {
    use nu_test_support::playground::Playground;

    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("autodir");

        let actual = nu!(
            cwd: dirs.test(),
            "
                autodir
                echo (pwd)
            "
        );

        assert!(actual.out.ends_with("autodir"));
    })
}

// FIXME: jt: we don't currently support autocd in testing
#[ignore]
#[test]
fn automatically_change_directory_with_trailing_slash_and_same_name_as_command() {
    use nu_test_support::playground::Playground;

    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("cd");

        let actual = nu!(
            cwd: dirs.test(),
            "
                cd/
                pwd
            "
        );

        assert!(actual.out.ends_with("cd"));
    })
}

#[test]
fn correctly_escape_external_arguments() {
    let actual = nu!("^nu --testbin cococo '$0'");

    assert_eq!(actual.out, "$0");
}

#[test]
fn escape_also_escapes_equals() {
    let actual = nu!("^MYFOONAME=MYBARVALUE");

    assert!(actual.err.contains("executable was not found"));
}

#[test]
fn execute_binary_in_string() {
    let actual = nu!(r#"
        let cmd = "nu"
        ^$"($cmd)" --testbin cococo "$0"
    "#);

    assert_eq!(actual.out, "$0");
}

#[test]
fn single_quote_dollar_external() {
    let actual = nu!("let author = 'JT'; nu --testbin cococo $'foo=($author)'");

    assert_eq!(actual.out, "foo=JT");
}

#[test]
fn redirects_custom_command_external() {
    let actual = nu!("def foo [] { nu --testbin cococo foo bar }; foo | str length");
    assert_eq!(actual.out, "7");
}

#[test]
fn passes_binary_data_between_externals() {
    let actual = nu!(cwd: "tests/fixtures/formats", "nu --testbin meowb sample.db | nu --testbin relay | hash sha256");

    assert_eq!(
        actual.out,
        "2f5050e7eea415c1f3d80b5d93355efd15043ec9157a2bb167a9e73f2ae651f2"
    )
}

#[test]
fn command_not_found_error_suggests_search_term() {
    // 'distinct' is not a command, but it is a search term for 'uniq'
    let actual = nu!("ls | distinct");
    assert!(actual.err.contains("uniq"));
}

#[test]
fn command_not_found_error_suggests_typo_fix() {
    let actual = nu!("benchmark { echo 'foo'}");
    assert!(actual.err.contains("timeit"));
}

#[test]
fn command_not_found_error_shows_not_found_1() {
    let actual = nu!(r#"
            export extern "foo" [];
            foo
        "#);
    assert!(actual.err.contains("'foo' was not found"));
}

#[test]
fn command_substitution_wont_output_extra_newline() {
    let actual = nu!(r#"
        with-env { FOO: "bar" } { echo $"prefix (nu --testbin echo_env FOO) suffix" }
        "#);
    assert_eq!(actual.out, "prefix bar suffix");

    let actual = nu!(r#"
        with-env { FOO: "bar" } { (nu --testbin echo_env FOO) }
        "#);
    assert_eq!(actual.out, "bar");
}

#[test]
fn basic_err_pipe_works() {
    let actual =
        nu!(r#"with-env { FOO: "bar" } { nu --testbin echo_env_stderr FOO e>| str length }"#);
    assert_eq!(actual.out, "3");
}

#[test]
fn basic_outerr_pipe_works() {
    let actual = nu!(
        r#"with-env { FOO: "bar" } { nu --testbin echo_env_mixed out-err FOO FOO o+e>| str length }"#
    );
    assert_eq!(actual.out, "7");
}

#[test]
fn err_pipe_with_failed_external_works() {
    let actual =
        nu!(r#"with-env { FOO: "bar" } { nu --testbin echo_env_stderr_fail FOO e>| str length }"#);
    assert_eq!(actual.out, "3");
}

#[test]
fn dont_run_glob_if_pass_variable_to_external() {
    Playground::setup("dont_run_glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("jt_likes_cake.txt"),
            EmptyFile("andres_likes_arepas.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), r#"let f = "*.txt"; nu --testbin nonu $f"#);

        assert_eq!(actual.out, "*.txt");
    })
}

#[test]
fn run_glob_if_pass_variable_to_external() {
    Playground::setup("run_glob_on_external", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("jt_likes_cake.txt"),
            EmptyFile("andres_likes_arepas.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), r#"let f = "*.txt"; nu --testbin nonu ...(glob $f)"#);

        assert!(actual.out.contains("jt_likes_cake.txt"));
        assert!(actual.out.contains("andres_likes_arepas.txt"));
    })
}

mod it_evaluation {
    use super::nu;
    use nu_test_support::fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed};
    use nu_test_support::{pipeline, playground::Playground};

    #[test]
    fn takes_rows_of_nu_value_strings() {
        Playground::setup("it_argument_test_1", |dirs, sandbox| {
            sandbox.with_files(vec![
                EmptyFile("jt_likes_cake.txt"),
                EmptyFile("andres_likes_arepas.txt"),
            ]);

            let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ls
                | sort-by name
                | get name
                | each { |it| nu --testbin cococo $it }
                | get 1
                "
            ));

            assert_eq!(actual.out, "jt_likes_cake.txt");
        })
    }

    #[test]
    fn takes_rows_of_nu_value_lines() {
        Playground::setup("it_argument_test_2", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContentToBeTrimmed(
                "nu_candies.txt",
                "
                    AndrásWithKitKatzz
                    AndrásWithKitKatz
                ",
            )]);

            let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                open nu_candies.txt
                | lines
                | each { |it| nu --testbin chop $it}
                | get 1
                "
            ));

            assert_eq!(actual.out, "AndrásWithKitKat");
        })
    }

    #[test]
    fn can_properly_buffer_lines_externally() {
        let actual = nu!("
                nu --testbin repeater c 8197 | lines | length
            ");

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
                "
                    open sample.toml
                    | nu --testbin cococo $in.nu_party_venue
                "
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
        let actual = nu!(pipeline(
            r#"
                nu --testbin nonu "wheres the nuline?" | length
            "#
        ));

        assert_eq!(actual.err, "");
    }

    #[test]
    fn does_not_block_indefinitely() {
        let stdout = nu!(pipeline(
            "
                ( nu --testbin iecho yes
                | nu --testbin chop
                | nu --testbin chop
                | lines
                | first )
            "
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
        let actual = nu!("
        nu --testbin cococo joturner@foo.bar.baz
        ");

        assert_eq!(actual.out, "joturner@foo.bar.baz");
    }

    //FIXME: jt: limitation in testing - can't use single ticks currently
    #[ignore]
    #[test]
    fn no_escaping_for_single_quoted_strings() {
        let actual = nu!(r#"
        nu --testbin cococo 'test "things"'
        "#);

        assert_eq!(actual.out, "test \"things\"");
    }

    #[rstest::rstest]
    #[case("sample.toml", r#""sample.toml""#)]
    #[case("a sample file.toml", r#""a sample file.toml""#)]
    //FIXME: jt: we don't currently support single ticks in tests
    //#[case("quote'mark.toml", r#""quote'mark.toml""#)]
    #[cfg_attr(
        not(target_os = "windows"),
        case(r#"quote"mark.toml"#, r#"$"quote(char double_quote)mark.toml""#)
    )]
    #[cfg_attr(not(target_os = "windows"), case("?mark.toml", r#""?mark.toml""#))]
    #[cfg_attr(not(target_os = "windows"), case("*.toml", r#""*.toml""#))]
    #[cfg_attr(not(target_os = "windows"), case("*.toml", "*.toml"))]
    #[case("$ sign.toml", r#""$ sign.toml""#)]
    fn external_arg_with_special_characters(#[case] path: &str, #[case] nu_path_argument: &str) {
        Playground::setup("external_arg_with_quotes", |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                path,
                r#"
                    nu_party_venue = "zion"
                "#,
            )]);

            let actual = nu!(
                cwd: dirs.test(), pipeline(
                &format!("
                    nu --testbin meow {nu_path_argument} | from toml | get nu_party_venue
                ")
            ));

            assert_eq!(actual.out, "zion");
        })
    }
}

mod nu_commands {
    use nu_test_support::playground::Playground;

    use super::nu;

    #[test]
    fn echo_internally_externally() {
        let actual = nu!(r#"
        nu -n -c "echo 'foo'"
        "#);

        assert_eq!(actual.out, "foo");
    }

    #[test]
    fn failed_with_proper_exit_code() {
        Playground::setup("external failed", |dirs, _sandbox| {
            let actual = nu!(cwd: dirs.test(), r#"
            nu -n -c "cargo build | complete | get exit_code"
            "#);

            // cargo for non rust project's exit code is 101.
            assert_eq!(actual.out, "101")
        })
    }

    #[test]
    fn better_arg_quoting() {
        let actual = nu!(r#"
        nu -n -c "\# '"
        "#);

        assert_eq!(actual.out, "");
    }

    #[test]
    fn command_list_arg_test() {
        let actual = nu!("
        nu ...['-n' '-c' 'version']
        ");

        assert!(actual.out.contains("version"));
        assert!(actual.out.contains("rust_version"));
        assert!(actual.out.contains("rust_channel"));
    }

    #[test]
    fn command_cell_path_arg_test() {
        let actual = nu!("
        nu ...([ '-n' '-c' 'version' ])
        ");

        assert!(actual.out.contains("version"));
        assert!(actual.out.contains("rust_version"));
        assert!(actual.out.contains("rust_channel"));
    }
}

mod nu_script {
    use super::nu;

    #[test]
    fn run_nu_script() {
        let actual = nu!(cwd: "tests/fixtures/formats", "
        nu -n script.nu
        ");

        assert_eq!(actual.out, "done");
    }

    #[test]
    fn run_nu_script_multiline() {
        let actual = nu!(cwd: "tests/fixtures/formats", "
        nu -n script_multiline.nu
        ");

        assert_eq!(actual.out, "23");
    }
}

mod tilde_expansion {
    use super::nu;

    #[test]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!("
            nu --testbin cococo  ~
        ");

        assert!(!actual.out.contains('~'));
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(r#"
                nu --testbin cococo  "1~1"
                "#);

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
                    EmptyFile("jt_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                let actual = nu!(
                cwd: dirs.test(), pipeline(
                "
                    nu --testbin cococo ...(ls | get name)
                "
                ));

                assert_eq!(
                    actual.out,
                    "andres_likes_arepas.txt ferris_not_here.txt jt_likes_cake.txt"
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
                    EmptyFile("jt_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                let actual = nu!(
                cwd: dirs.test(), pipeline(
                "
                    nu --testbin cococo (ls | sort-by name | get name).1
                "
                ));

                assert_eq!(actual.out, "ferris_not_here.txt");
            },
        )
    }

    #[cfg(not(windows))]
    #[test]
    fn string_interpolation_with_an_external_command() {
        Playground::setup(
            "string_interpolation_with_an_external_command",
            |dirs, sandbox| {
                sandbox.mkdir("cd");

                sandbox.with_files(vec![EmptyFile("cd/jt_likes_cake.txt")]);

                let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                    nu --testbin cococo $"(pwd)/cd"
                "#
                ));

                assert!(actual.out.contains("cd"));
            },
        )
    }

    #[test]
    fn semicolons_are_sanitized_before_passing_to_subshell() {
        let actual = nu!("nu --testbin cococo \"a;b\"");

        assert_eq!(actual.out, "a;b");
    }

    #[test]
    fn ampersands_are_sanitized_before_passing_to_subshell() {
        let actual = nu!("nu --testbin cococo \"a&b\"");

        assert_eq!(actual.out, "a&b");
    }

    #[cfg(not(windows))]
    #[test]
    fn subcommands_are_sanitized_before_passing_to_subshell() {
        let actual = nu!("nu --testbin cococo \"$(ls)\"");

        assert_eq!(actual.out, "$(ls)");
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_arguments_are_sanitized_even_if_coming_from_other_commands() {
        let actual = nu!("nu --testbin cococo (echo \"a;&$(hello)\")");

        assert_eq!(actual.out, "a;&$(hello)");
    }
}
