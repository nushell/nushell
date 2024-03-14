#[cfg(not(windows))]
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn better_empty_redirection() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            ls | each { |it| nu --testbin cococo $it.name } | ignore
        "
    ));

    eprintln!("out: {}", actual.out);

    assert!(!actual.out.contains('2'));
}

#[cfg(not(windows))]
#[test]
fn explicit_glob() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls | glob '*.txt' | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[cfg(not(windows))]
#[test]
fn bare_word_expand_path_glob() {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ^ls *.txt
            "
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(not(windows))]
#[test]
fn backtick_expand_path_glob() {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls `*.txt`
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(not(windows))]
#[test]
fn single_quote_does_not_expand_path_glob() {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls '*.txt'
            "#
        ));

        assert!(actual.err.contains("No such file or directory"));
    })
}

#[cfg(not(windows))]
#[test]
fn double_quote_does_not_expand_path_glob() {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls "*.txt"
            "#
        ));

        assert!(actual.err.contains("No such file or directory"));
    })
}

#[cfg(not(windows))]
#[test]
fn failed_command_with_semicolon_will_not_execute_following_cmds() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ^ls *.abc; echo done
            "
        ));

        assert!(!actual.out.contains("done"));
    })
}

#[test]
fn external_args_with_quoted() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu --testbin cococo "foo=bar 'hi'"
            "#
        ));

        assert_eq!(actual.out, "foo=bar 'hi'");
    })
}

#[cfg(not(windows))]
#[test]
fn external_arg_with_long_flag_value_quoted() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^echo --foo='bar'
            "#
        ));

        assert_eq!(actual.out, "--foo=bar");
    })
}

#[test]
fn external_arg_with_variable_name() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                let dump_command = "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'";
                nu --testbin nonu $dump_command
            "#
        ));

        assert_eq!(
            actual.out,
            r#"PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'"#
        );
    })
}

#[test]
fn external_command_escape_args() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu --testbin cococo "\"abcd"
            "#
        ));

        assert_eq!(actual.out, r#""abcd"#);
    })
}

#[test]
fn external_command_not_expand_tilde_with_quotes() {
    Playground::setup(
        "external command not expand tilde with quotes",
        |dirs, _| {
            let actual = nu!(cwd: dirs.test(), pipeline(r#"nu --testbin nonu "~""#));
            assert_eq!(actual.out, r#"~"#);
        },
    )
}

#[test]
fn external_command_expand_tilde_with_back_quotes() {
    Playground::setup(
        "external command not expand tilde with quotes",
        |dirs, _| {
            let actual = nu!(cwd: dirs.test(), pipeline(r#"nu --testbin nonu `~`"#));
            assert!(!actual.out.contains('~'));
        },
    )
}

#[test]
fn external_command_receives_raw_binary_data() {
    Playground::setup("external command receives raw binary data", |dirs, _| {
        let actual =
            nu!(cwd: dirs.test(), pipeline("0x[deadbeef] | nu --testbin input_bytes_length"));
        assert_eq!(actual.out, r#"4"#);
    })
}

#[cfg(windows)]
#[test]
fn failed_command_with_semicolon_will_not_execute_following_cmds_windows() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ^cargo asdf; echo done
            "
        ));

        assert!(!actual.out.contains("done"));
    })
}

#[cfg(windows)]
#[test]
fn can_run_batch_files() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows batch file", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "foo.cmd",
            r#"
                @echo off
                echo Hello World
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), pipeline("foo.cmd"));
        assert!(actual.out.contains("Hello World"));
    });
}

#[cfg(windows)]
#[test]
fn can_run_batch_files_without_cmd_extension() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup(
        "run a Windows batch file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "foo.cmd",
                r#"
                @echo off
                echo Hello World
            "#,
            )]);

            let actual = nu!(cwd: dirs.test(), pipeline("foo"));
            assert!(actual.out.contains("Hello World"));
        },
    );
}

#[cfg(windows)]
#[test]
fn can_run_batch_files_without_bat_extension() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup(
        "run a Windows batch file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(vec![FileWithContent(
                "foo.bat",
                r#"
                @echo off
                echo Hello World
            "#,
            )]);

            let actual = nu!(cwd: dirs.test(), pipeline("foo"));
            assert!(actual.out.contains("Hello World"));
        },
    );
}

#[test]
fn quotes_trimmed_when_shelling_out() {
    // regression test for a bug where we weren't trimming quotes around string args before shelling out to cmd.exe
    let actual = nu!(pipeline(
        r#"
            nu --testbin cococo "foo"
        "#
    ));

    assert_eq!(actual.out, "foo");
}

#[cfg(not(windows))]
#[test]
fn redirect_combine() {
    Playground::setup("redirect_combine", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                run-external sh ...[-c 'echo Foo; echo >&2 Bar'] o+e>| print
            "#
        ));

        // Lines are collapsed in the nu! macro
        assert_eq!(actual.out, "FooBar");
    });
}
