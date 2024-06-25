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

#[test]
fn explicit_glob() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo ('*.txt' | into glob)
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[test]
fn bare_word_expand_path_glob() {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                ^nu --testbin cococo *.txt
            "
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[test]
fn backtick_expand_path_glob() {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo `*.txt`
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[test]
fn single_quote_does_not_expand_path_glob() {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo '*.txt'
            "#
        ));

        assert_eq!(actual.out, "*.txt");
    })
}

#[test]
fn double_quote_does_not_expand_path_glob() {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo "*.txt"
            "#
        ));

        assert_eq!(actual.out, "*.txt");
    })
}

#[test]
fn failed_command_with_semicolon_will_not_execute_following_cmds() {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                nu --testbin fail; echo done
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
fn external_arg_with_option_like_embedded_quotes() {
    // TODO: would be nice to make this work with cococo, but arg parsing interferes
    Playground::setup(
        "external arg with option like embedded quotes",
        |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                ^echo --foo='bar' -foo='bar'
            "#
            ));

            assert_eq!(actual.out, "--foo=bar -foo=bar");
        },
    )
}

#[test]
fn external_arg_with_non_option_like_embedded_quotes() {
    Playground::setup(
        "external arg with non option like embedded quotes",
        |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                r#"
                ^nu --testbin cococo foo='bar' 'foo'=bar
            "#
            ));

            assert_eq!(actual.out, "foo=bar foo=bar");
        },
    )
}

#[test]
fn external_arg_with_string_interpolation() {
    Playground::setup("external arg with string interpolation", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo foo=(2 + 2) $"foo=(2 + 2)" foo=$"(2 + 2)"
            "#
        ));

        assert_eq!(actual.out, "foo=4 foo=4 foo=4");
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
fn external_command_ndots_args() {
    let actual = nu!(r#"
        nu --testbin cococo foo/. foo/.. foo/... foo/./bar foo/../bar foo/.../bar ./bar ../bar .../bar
    "#);

    assert_eq!(
        actual.out,
        if cfg!(windows) {
            // Windows is a bit weird right now, where if ndots has to fix something it's going to
            // change everything to backslashes too. Would be good to fix that
            r"foo/. foo/.. foo\..\.. foo/./bar foo/../bar foo\..\..\bar ./bar ../bar ..\..\bar"
        } else {
            r"foo/. foo/.. foo/../.. foo/./bar foo/../bar foo/../../bar ./bar ../bar ../../bar"
        }
    );
}

#[test]
fn external_command_url_args() {
    // If ndots is not handled correctly, we can lose the double forward slashes that are needed
    // here
    let actual = nu!(r#"
        nu --testbin cococo http://example.com http://example.com/.../foo //foo
    "#);

    assert_eq!(
        actual.out,
        "http://example.com http://example.com/.../foo //foo"
    );
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "only runs on Linux, where controlling the HOME var is reliable"
)]
fn external_command_expand_tilde() {
    Playground::setup("external command expand tilde", |dirs, _| {
        // Make a copy of the nu executable that we can use
        let mut src = std::fs::File::open(nu_test_support::fs::binaries().join("nu"))
            .expect("failed to open nu");
        let mut dst = std::fs::File::create_new(dirs.test().join("test_nu"))
            .expect("failed to create test_nu file");
        std::io::copy(&mut src, &mut dst).expect("failed to copy data for nu binary");

        // Make test_nu have the same permissions so that it's executable
        dst.set_permissions(
            src.metadata()
                .expect("failed to get nu metadata")
                .permissions(),
        )
        .expect("failed to set permissions on test_nu");

        // Close the files
        drop(dst);
        drop(src);

        let actual = nu!(
            envs: vec![
                ("HOME".to_string(), dirs.test().to_string_lossy().into_owned()),
            ],
            r#"
                ^~/test_nu --testbin cococo hello
            "#
        );
        assert_eq!(actual.out, "hello");
    })
}

#[test]
fn external_arg_expand_tilde() {
    Playground::setup("external arg expand tilde", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^nu --testbin cococo ~/foo ~/(2 + 2)
            "#
        ));

        let home = dirs_next::home_dir().expect("failed to find home dir");

        assert_eq!(
            actual.out,
            format!(
                "{} {}",
                home.join("foo").display(),
                home.join("4").display()
            )
        );
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
fn can_run_batch_files() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows batch file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
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
            sandbox.with_files(&[FileWithContent(
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
            sandbox.with_files(&[FileWithContent(
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
