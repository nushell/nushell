use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn better_empty_redirection() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            ls | each { |it| nu --testbin cococo $it.name }
        "#
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
            r#"
                ^ls *.txt
            "#
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
            r#"
                ^ls *.abc; echo done
            "#
        ));

        assert!(!actual.out.contains("done"));
    })
}

#[cfg(not(windows))]
#[test]
fn external_args_with_quoted() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^echo "foo=bar 'hi'"
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

#[cfg(not(windows))]
#[test]
fn external_command_escape_args() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^echo "\"abcd"
            "#
        ));

        assert_eq!(actual.out, r#""abcd"#);
    })
}

#[cfg(windows)]
#[test]
fn explicit_glob_windows() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir | glob '*.txt' | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[cfg(windows)]
#[test]
fn bare_word_expand_path_glob_windows() {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir *.txt
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(windows)]
#[test]
fn failed_command_with_semicolon_will_not_execute_following_cmds_windows() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^cargo asdf; echo done
            "#
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
