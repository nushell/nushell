use nu_test_support::fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::{assert_eq, assert_matches};
use rstest::rstest;

#[test]
fn shows_error_for_command_not_found() -> Result {
    test()
        .run("ferris_is_not_here.exe")
        .expect_error_code_eq("nu::shell::external_command")
}

#[test]
fn shows_error_for_command_not_found_in_pipeline() -> Result {
    test()
        .run("ferris_is_not_here.exe | echo done")
        .expect_error_code_eq("nu::shell::external_command")
}

#[ignore]
// jt: we can't test this using the -c workaround currently
// piet: auto cd seems to not be available for regular scripts, so we maybe have to add
//       that to the tester?
#[test]
fn automatically_change_directory() -> Result {
    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("autodir");

        test()
            .cwd(dirs.test())
            .run("autodir; echo (pwd)")
            .expect_value_eq(dirs.test().join("autodir").to_string_lossy())
    })
}

// FIXME: jt: we don't currently support autocd in testing
#[ignore]
#[test]
fn automatically_change_directory_with_trailing_slash_and_same_name_as_command() -> Result {
    Playground::setup("cd_test_5_1", |dirs, sandbox| {
        sandbox.mkdir("cd");

        test()
            .cwd(dirs.test())
            .run("cd/; pwd")
            .expect_value_eq(dirs.test().join("cd").to_string_lossy())
    })
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn pass_dot_as_external_arguments() -> Result {
    test().run("cococo .").expect_value_eq(".")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn correctly_escape_external_arguments() -> Result {
    test().run("^cococo '$0'").expect_value_eq("$0")
}

#[test]
fn escape_also_escapes_equals() -> Result {
    let err = test().run("^MYFOONAME=MYBARVALUE").expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `MYFOONAME=MYBARVALUE` not found");
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn execute_binary_in_string() -> Result {
    let code = r#"
        let cmd = "cococo"
        ^$"($cmd)" "$0"
    "#;

    test().run(code).expect_value_eq("$0")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn single_quote_dollar_external() -> Result {
    test()
        .run("let author = 'JT'; cococo $'foo=($author)'")
        .expect_value_eq("foo=JT")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn redirects_custom_command_external() -> Result {
    test()
        .run("def foo [] { cococo foo bar }; foo | str length")
        .expect_value_eq(7)
}

#[test]
#[deps(TESTBIN_MEOWB, TESTBIN_RELAY)]
fn passes_binary_data_between_externals() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("meowb sample.db | relay | hash sha256")
        .expect_value_eq("2f5050e7eea415c1f3d80b5d93355efd15043ec9157a2bb167a9e73f2ae651f2")
}

#[test]
fn command_not_found_error_suggests_search_term() -> Result {
    let err = test().run("ls | distinct").expect_shell_error()?;
    // 'distinct' is not a command, but it is a search term for 'uniq'
    assert_matches!(err, ShellError::ExternalCommand { help, .. } if help.contains("uniq"));
    Ok(())
}

#[test]
fn command_not_found_error_suggests_typo_fix() -> Result {
    let err = test().run("benchmark { echo 'foo'}").expect_shell_error()?;
    assert_matches!(err, ShellError::RemovedCommand { replacement, .. } if replacement == "timeit");
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn command_not_found_error_recognizes_non_executable_file() -> Result {
    let err = test().run("./Cargo.toml").expect_shell_error()?;
    let expected = "`./Cargo.toml` refers to a file that is not executable. Did you forget to set execute permissions?";
    assert_matches!(err, ShellError::ExternalCommand {help, ..} if help == expected);
    Ok(())
}

#[test]
fn command_not_found_error_shows_not_found_1() -> Result {
    let code = r#"
        export extern "foo" []
        foo
    "#;

    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `foo` not found");
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV)]
fn command_substitution_wont_output_extra_newline() -> Result {
    test()
        .run(r#"with-env { FOO: "bar" } { echo $"prefix (echo_env FOO) suffix" }"#)
        .expect_value_eq("prefix bar suffix")?;

    test()
        .run(r#"with-env { FOO: "bar" } { (echo_env FOO) }"#)
        .expect_value_eq("bar")
}

#[rstest]
#[case::err_pipe_long("err>|")]
#[case::err_pipe_short("e>|")]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn basic_err_pipe_works(#[case] redirection: &str) -> Result {
    let code =
        format!(r#"with-env {{ FOO: "bar" }} {{ echo_env_stderr FOO {redirection} str length }}"#);

    test().run(code).expect_value_eq(3)
}

#[rstest]
#[case::out_err_pipe_long("out+err>|")]
#[case::err_out_pipe_long("err+out>|")]
#[case::out_err_pipe_short("o+e>|")]
#[case::err_out_pipe_short("e+o>|")]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn basic_outerr_pipe_works(#[case] redirection: &str) -> Result {
    let code = format!(
        r#"with-env {{ FOO: "bar" }} {{ echo_env_mixed out-err FOO FOO {redirection} str length }}"#
    );

    test().run(code).expect_value_eq(7)
}

#[test]
#[deps(TESTBIN_NONU)]
fn dont_run_glob_if_pass_variable_to_external() -> Result {
    Playground::setup("dont_run_glob", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("jt_likes_cake.txt"),
            EmptyFile("andres_likes_arepas.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run(r#"let f = "*.txt"; nonu $f"#)
            .expect_value_eq("*.txt")
    })
}

#[test]
#[deps(TESTBIN_NONU)]
fn run_glob_if_pass_variable_to_external() -> Result {
    Playground::setup("run_glob_on_external", |dirs, sandbox| -> Result {
        sandbox.with_files(&[
            EmptyFile("jt_likes_cake.txt"),
            EmptyFile("andres_likes_arepas.txt"),
        ]);

        let out: String = test()
            .cwd(dirs.test())
            .run(r#"let f = "*.txt"; nonu ...(glob $f)"#)?;

        assert_contains("jt_likes_cake.txt", &out);
        assert_contains("andres_likes_arepas.txt", out);
        Ok(())
    })
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn subexpression_does_not_implicitly_capture() -> Result {
    let result: CompleteResult = test().run(r#"nu -n -c "(cococo); null" | complete"#)?;

    assert_eq!(result.stdout.trim(), "cococo");
    Ok(())
}

mod it_evaluation {
    use super::*;

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn takes_rows_of_nu_value_strings() -> Result {
        Playground::setup("it_argument_test_1", |dirs, sandbox| {
            sandbox.with_files(&[
                EmptyFile("jt_likes_cake.txt"),
                EmptyFile("andres_likes_arepas.txt"),
            ]);

            let code = "
                ls
                | sort-by name
                | get name
                | each {|it| cococo $it }
                | get 1
            ";

            test()
                .cwd(dirs.test())
                .run(code)
                .expect_value_eq("jt_likes_cake.txt")
        })
    }

    #[test]
    #[deps(TESTBIN_CHOP)]
    fn takes_rows_of_nu_value_lines() -> Result {
        Playground::setup("it_argument_test_2", |dirs, sandbox| {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "nu_candies.txt",
                "
                    AndrásWithKitKatzz
                    AndrásWithKitKatz
                ",
            )]);

            let code = "
                open nu_candies.txt
                | lines
                | each {|it| chop $it }
                | get 1
            ";

            test()
                .cwd(dirs.test())
                .run(code)
                .expect_value_eq("AndrásWithKitKat")
        })
    }

    #[test]
    #[deps(TESTBIN_REPEATER)]
    fn can_properly_buffer_lines_externally() -> Result {
        test()
            .run("repeater c 8197 | lines | length")
            .expect_value_eq(1)
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn supports_fetching_given_a_column_path_to_it() -> Result {
        Playground::setup("it_argument_test_3", |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "sample.toml",
                r#"
                    nu_party_venue = "zion"
                "#,
            )]);

            let code = "
                open sample.toml
                | cococo $in.nu_party_venue
            ";

            test().cwd(dirs.test()).run(code).expect_value_eq("zion")
        })
    }
}

mod stdin_evaluation {
    use super::*;

    #[test]
    #[deps(TESTBIN_NONU)]
    fn does_not_panic_with_no_newline_in_stream() -> Result {
        let _: i64 = test().run(r#"nonu "where's the nuline?" | length"#)?;
        Ok(())
    }

    #[test]
    #[deps(TESTBIN_IECHO, TESTBIN_CHOP)]
    fn does_not_block_indefinitely() -> Result {
        test()
            .run("(iecho yes | chop | chop | lines | first)")
            .expect_value_eq("y")
    }
}

mod external_words {
    use super::*;

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn relaxed_external_words() -> Result {
        test()
            .run("cococo joturner@foo.bar.baz")
            .expect_value_eq("joturner@foo.bar.baz")
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn raw_string_as_external_argument() -> Result {
        test().run("cococo r#'asdf'#").expect_value_eq("asdf")
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn no_escaping_for_single_quoted_strings() -> Result {
        test()
            .run(r#"cococo 'test "things"'"#)
            .expect_value_eq("test \"things\"")
    }

    #[rstest]
    #[case::simple_name("sample.toml", r#""sample.toml""#)]
    #[case::space_in_name("a sample file.toml", r#""a sample file.toml""#)]
    //FIXME: jt: we don't currently support single ticks in tests
    //#[case("quote'mark.toml", r#""quote'mark.toml""#)]
    #[cfg_attr(
        not(windows),
        case::double_quote_in_name(
            r#"quote"mark.toml"#,
            r#"$"quote(char double_quote)mark.toml""#
        )
    )]
    #[cfg_attr(
        not(windows),
        case::question_mark_in_name("?mark.toml", r#""?mark.toml""#)
    )]
    #[cfg_attr(not(windows), case::quoted_glob("*.toml", r#""*.toml""#))]
    #[cfg_attr(not(windows), case::bare_glob("*.toml", "*.toml"))]
    #[case::dollar_sign_in_name("$ sign.toml", r#""$ sign.toml""#)]
    #[nu_test_support::test]
    #[deps(TESTBIN_MEOW)]
    fn external_arg_with_special_characters(
        #[case] path: &str,
        #[case] nu_path_argument: &str,
    ) -> Result {
        Playground::setup("external_arg_with_quotes", |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                path,
                r#"
                    nu_party_venue = "zion"
                "#,
            )]);

            test()
                .cwd(dirs.test())
                .run(format!(
                    "meow {nu_path_argument} | from toml | get nu_party_venue",
                ))
                .expect_value_eq("zion")
        })
    }
}

mod nu_commands {
    use super::*;

    #[test]
    #[deps(NU)]
    fn echo_internally_externally() -> Result {
        test()
            .run(r#"nu -n -c "echo 'foo'""#)
            .expect_value_eq("foo")
    }

    #[test]
    #[deps(TESTBIN_FAIL)]
    fn failed_with_proper_exit_code() -> Result {
        test()
            .run("fail 101 | complete | get exit_code")
            .expect_value_eq(101)
    }

    #[test]
    #[deps(NU)]
    fn better_arg_quoting() -> Result {
        test().run(r#"nu -n -c "\# '""#).expect_value_eq("")
    }

    #[test]
    #[deps(NU)]
    fn command_list_arg_test() -> Result {
        let out: String = test().run("nu ...['-n' '-c' 'version']")?;

        assert_contains("version", &out);
        assert_contains("rust_version", &out);
        assert_contains("rust_channel", out);
        Ok(())
    }

    #[test]
    #[deps(NU)]
    fn command_cell_path_arg_test() -> Result {
        let out: String = test().run("nu ...([ '-n' '-c' 'version' ])")?;

        assert_contains("version", &out);
        assert_contains("rust_version", &out);
        assert_contains("rust_channel", out);
        Ok(())
    }
}

mod nu_script {
    use super::*;

    #[test]
    #[deps(NU)]
    fn run_nu_script() -> Result {
        test()
            .cwd("tests/fixtures/formats")
            .run("nu -n script.nu")
            .expect_value_eq("done")
    }

    #[test]
    #[deps(NU)]
    fn run_nu_script_multiline() -> Result {
        test()
            .cwd("tests/fixtures/formats")
            .run("nu -n script_multiline.nu")
            .expect_value_eq("2\n3")
    }
}

mod tilde_expansion {
    use super::*;

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() -> Result {
        let out: String = test().run("cococo ~")?;

        assert_contains_not("~", out);
        Ok(())
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() -> Result {
        test().run(r#"cococo "1~1""#).expect_value_eq("1~1")
    }
}

mod external_command_arguments {
    use super::*;

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn expands_table_of_primitives_to_positional_arguments() -> Result {
        Playground::setup(
            "expands_table_of_primitives_to_positional_arguments",
            |dirs, sandbox| {
                sandbox.with_files(&[
                    EmptyFile("jt_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                test()
                    .cwd(dirs.test())
                    .run("cococo ...(ls | get name)")
                    .expect_value_eq(
                        "andres_likes_arepas.txt ferris_not_here.txt jt_likes_cake.txt",
                    )
            },
        )
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn proper_subexpression_paths_in_external_args() -> Result {
        Playground::setup(
            "expands_table_of_primitives_to_positional_arguments",
            |dirs, sandbox| {
                sandbox.with_files(&[
                    EmptyFile("jt_likes_cake.txt"),
                    EmptyFile("andres_likes_arepas.txt"),
                    EmptyFile("ferris_not_here.txt"),
                ]);

                test()
                    .cwd(dirs.test())
                    .run("cococo (ls | sort-by name | get name).1")
                    .expect_value_eq("ferris_not_here.txt")
            },
        )
    }

    #[cfg(not(windows))]
    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn string_interpolation_with_an_external_command() -> Result {
        Playground::setup(
            "string_interpolation_with_an_external_command",
            |dirs, sandbox| {
                sandbox.mkdir("cd");
                sandbox.with_files(&[EmptyFile("cd/jt_likes_cake.txt")]);

                let out: String = test().cwd(dirs.test()).run(r#"cococo $"(pwd)/cd""#)?;

                assert_contains("cd", out);
                Ok(())
            },
        )
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn semicolons_are_sanitized_before_passing_to_subshell() -> Result {
        test().run("cococo \"a;b\"").expect_value_eq("a;b")
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn ampersands_are_sanitized_before_passing_to_subshell() -> Result {
        test().run("cococo \"a&b\"").expect_value_eq("a&b")
    }

    #[cfg(not(windows))]
    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn subcommands_are_sanitized_before_passing_to_subshell() -> Result {
        test().run("cococo \"$(ls)\"").expect_value_eq("$(ls)")
    }

    #[cfg(not(windows))]
    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn shell_arguments_are_sanitized_even_if_coming_from_other_commands() -> Result {
        test()
            .run("cococo (echo \"a;&$(hello)\")")
            .expect_value_eq("a;&$(hello)")
    }

    #[test]
    #[deps(TESTBIN_COCOCO)]
    fn remove_quotes_in_shell_arguments() -> Result {
        test()
            .run("cococo expression='-r -w'")
            .expect_value_eq("expression=-r -w")?;
        test()
            .run(r#"cococo expression="-r -w""#)
            .expect_value_eq("expression=-r -w")?;
        test()
            .run("cococo expression='-r -w'")
            .expect_value_eq("expression=-r -w")?;
        test()
            .run(r#"cococo expression="-r\" -w""#)
            .expect_value_eq(r#"expression=-r" -w"#)?;
        test()
            .run(r#"cococo expression='-r\" -w'"#)
            .expect_value_eq(r#"expression=-r\" -w"#)
    }
}

#[test]
#[deps(NU)]
fn exit_code_stops_execution_closure() -> Result {
    let result: CompleteResult =
        test().run(r#"nu -n -c "[1 2] | each {|x| nu -c $'exit ($x)'; print $x }" | complete"#)?;

    assert_eq!(result.stdout, "");
    assert_contains("exited with code 1", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn exit_code_stops_execution_custom_command() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -n -c "def cmd [] { nu -c 'exit 42'; 'ok1' }; cmd; print 'ok2'" | complete"#)?;

    assert_eq!(result.stdout, "");
    assert_contains_not("exited with code 42", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn exit_code_stops_execution_for_loop() -> Result {
    let result: CompleteResult =
        test().run(r#"nu -n -c "for x in [0 1] { nu -c 'exit 42'; print $x }" | complete"#)?;

    assert_eq!(result.stdout, "");
    assert_contains_not("exited with code 42", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn display_error_with_exit_code_stops() -> Result {
    Playground::setup("errexit", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "tmp_env.nu",
            "$env.config.display_errors.exit_code = true",
        )]);

        let result: CompleteResult = test().cwd(dirs.test()).run(
            r#"nu --env-config tmp_env.nu --no-std-lib --error-style plain --commands "def cmd [] { nu -c 'exit 42'; 'ok1' }; cmd; print 'ok2'" | complete"#,
        )?;

        assert_contains("exited with code", result.stderr);
        assert_eq!(result.stdout, "");
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn display_error_exit_code_stops_execution_for_loop() -> Result {
    Playground::setup("errexit", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "tmp_env.nu",
            "$env.config.display_errors.exit_code = true",
        )]);

        let result: CompleteResult = test().cwd(dirs.test()).run(
            r#"nu --env-config tmp_env.nu --no-std-lib --error-style plain --commands "for x in [0 1] { nu -c 'exit 42'; print $x }" | complete"#,
        )?;

        assert_contains("exited with code", result.stderr);
        assert_eq!(result.stdout, "");
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn arg_dont_run_subcommand_if_surrounded_with_quote() -> Result {
    test()
        .run("cococo `(echo aa)`")
        .expect_value_eq("(echo aa)")?;
    test()
        .run("cococo \"(echo aa)\"")
        .expect_value_eq("(echo aa)")?;
    test()
        .run("cococo '(echo aa)'")
        .expect_value_eq("(echo aa)")
}

#[test]
#[deps(NU, TESTBIN_FAIL)]
fn external_error_with_backtrace() -> Result {
    Playground::setup("external error with backtrace", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("tmp_env.nu", "$env.NU_BACKTRACE = 1")]);

        let result: CompleteResult = test().cwd(dirs.test()).run(
            r#"nu --env-config tmp_env.nu --no-std-lib --error-style plain --commands "def a [x] { if $x == 3 { fail }}; def b [] { a 1; a 3; a 2 }; b" | complete"#,
        )?;

        assert_eq!(
            result
                .stderr
                .matches("diagnostic code: chained_error")
                .count(),
            1
        );
        assert_contains("non_zero_exit_code", &result.stderr);
        assert_eq!(result.stderr.matches("eval_block_with_input").count(), 1);

        let result: CompleteResult = test().cwd(dirs.test()).run(
            r#"nu --env-config tmp_env.nu --no-std-lib --error-style plain --commands "fail" | complete"#,
        )?;

        assert_eq!(
            result
                .stderr
                .matches("diagnostic code: chained_error")
                .count(),
            0
        );
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn sub_external_expression_with_and_op_should_raise_proper_error() -> Result {
    let err = test().run("(cococo false) and true").expect_shell_error()?;
    assert_contains(
        "The 'and' operator does not work on values of type 'string'",
        err.to_string(),
    );
    Ok(())
}

#[test]
#[deps(NU)]
fn bad_config_file_restrict_cmd_running_with_commands() -> Result {
    Playground::setup("bad config file", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent("tmp_env.nu", "errorcmd")]);

        let result: CompleteResult = test()
            .cwd(dirs.test())
            .run(r#"nu --env-config tmp_env.nu --no-std-lib --error-style plain --commands "print bbb" | complete"#)?;

        assert_contains("Command `errorcmd` not found", result.stderr);
        assert_contains_not("bbb", result.stdout);
        assert_ne!(result.exit_code, 0);
        Ok(())
    })?;

    let result: CompleteResult = test().run(
        r#"nu --env-config not_exists.nu --no-std-lib --error-style plain --commands "print bbb" | complete"#,
    )?;

    assert_contains("File not found: not_exists.nu", result.stderr);
    assert_contains_not("bbb", result.stdout);
    assert_ne!(result.exit_code, 0);
    Ok(())
}

// FIXME: ignore these cases for now, the value inside a pipeline
// makes all previous exit status untracked.
// #[case("fail 10 | fail 20 | 10", 10)]
// #[case("fail 20 | 10 | fail", 20)]
// #[case("30 | fail | fail 30", 1)]
#[rstest]
#[case::fail_before_print("fail | print aa", 1)]
#[case::successful_external_before_print("nonu a | print bb", 0)]
#[case::fail_then_success_then_print("fail 30 | nonu a | print aa", 30)]
#[case::prints_then_fail("print aa | print cc | fail 40", 40)]
#[case::fail_then_print_then_fail_default("fail 20 | print aa | fail", 1)]
#[case::fail_default_then_print_then_fail("fail | print aa | fail 20", 20)]
#[case::let_captures_failed_pipeline("let x = fail 20 | into int", 20)]
#[nu_test_support::test]
#[deps(NU, TESTBIN_FAIL, TESTBIN_NONU)]
fn pipefail_feature(#[case] inp: &str, #[case] expect_code: i64) -> Result {
    let result: CompleteResult = test().run_with_data(
        "let code = $in; nu --no-config-file --no-std-lib --experimental-options=pipefail=true --commands $code | complete",
        inp,
    )?;

    assert_eq!(result.exit_code, expect_code);
    Ok(())
}
