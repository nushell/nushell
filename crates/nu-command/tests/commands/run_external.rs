use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use rstest::{fixture, rstest};
use rstest_reuse::*;

#[fixture]
#[once]
fn nu_bin() -> String {
    nu_test_support::fs::executable_path()
        .to_string_lossy()
        .to_string()
}

// Template for run-external test to ensure tests work when calling
// the binary directly, using the caret operator, and when using
// the run-external command
#[template]
#[rstest]
#[case("")]
#[case("^")]
#[case("run-external ")]
fn run_external_prefixes(nu_bin: &str, #[case] prefix: &str) {}

#[apply(run_external_prefixes)]
fn better_empty_redirection(nu_bin: &str, prefix: &str) {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        format!("ls | each {{ |it| {prefix}{nu_bin} `--testbin` cococo $it.name }} | ignore")
    );

    eprintln!("out: {}", actual.out);

    assert!(!actual.out.contains('2'));
}

#[apply(run_external_prefixes)]
fn explicit_glob(nu_bin: &str, prefix: &str) {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo ('*.txt' | into glob)"#)
        );

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[apply(run_external_prefixes)]
fn bare_word_expand_path_glob(nu_bin: &str, prefix: &str) {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            format!("{prefix}{nu_bin} `--testbin` cococo *.txt")
        );

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[apply(run_external_prefixes)]
fn backtick_expand_path_glob(nu_bin: &str, prefix: &str) {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo `*.txt`"#)
        );

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[apply(run_external_prefixes)]
fn single_quote_does_not_expand_path_glob(nu_bin: &str, prefix: &str) {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo '*.txt'"#)
        );

        assert_eq!(actual.out, "*.txt");
    })
}

#[apply(run_external_prefixes)]
fn double_quote_does_not_expand_path_glob(nu_bin: &str, prefix: &str) {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo "*.txt""#)
        );

        assert_eq!(actual.out, "*.txt");
    })
}

#[apply(run_external_prefixes)]
fn failed_command_with_semicolon_will_not_execute_following_cmds(nu_bin: &str, prefix: &str) {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("{prefix}{nu_bin} `--testbin` fail; echo done")
        );

        assert!(!actual.out.contains("done"));
    })
}

#[apply(run_external_prefixes)]
fn external_args_with_quoted(nu_bin: &str, prefix: &str) {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo "foo=bar 'hi'""#)
        );

        assert_eq!(actual.out, "foo=bar 'hi'");
    })
}

// don't use template for this once since echo with no prefix is an internal command
// and arguments flags are treated as arguments to run-external
// (but wrapping them in quotes defeats the point of test)
#[cfg(not(windows))]
#[test]
fn external_arg_with_option_like_embedded_quotes() {
    // TODO: would be nice to make this work with cococo, but arg parsing interferes
    Playground::setup(
        "external arg with option like embedded quotes",
        |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(),
                r#"^echo --foo='bar' -foo='bar'"#,
            );

            assert_eq!(actual.out, "--foo=bar -foo=bar");
        },
    )
}

// FIXME: parser complains about invalid characters after single quote
#[rstest]
#[case("")]
#[case("^")]
fn external_arg_with_non_option_like_embedded_quotes(nu_bin: &str, #[case] prefix: &str) {
    Playground::setup(
        "external arg with non option like embedded quotes",
        |dirs, _| {
            let actual = nu!(
                cwd: dirs.test(),
                format!(r#"{prefix}{nu_bin} `--testbin` cococo foo='bar' 'foo'=bar"#),
            );

            assert_eq!(actual.out, "foo=bar foo=bar");
        },
    )
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[rstest]
#[case("")]
#[case("^")]
fn external_arg_with_string_interpolation(nu_bin: &str, #[case] prefix: &str) {
    Playground::setup("external arg with string interpolation", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo foo=(2 + 2) $"foo=(2 + 2)" foo=$"(2 + 2)""#)
        );

        assert_eq!(actual.out, "foo=4 foo=4 foo=4");
    })
}

#[apply(run_external_prefixes)]
fn external_arg_with_variable_name(nu_bin: &str, prefix: &str) {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"
                let dump_command = "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'";
                {prefix}{nu_bin} `--testbin` nonu $dump_command
            "#)
        );

        assert_eq!(
            actual.out,
            r#"PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'"#
        );
    })
}

#[apply(run_external_prefixes)]
fn external_command_escape_args(nu_bin: &str, prefix: &str) {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo "\"abcd""#)
        );

        assert_eq!(actual.out, r#""abcd"#);
    })
}

#[apply(run_external_prefixes)]
fn external_command_ndots_args(nu_bin: &str, prefix: &str) {
    let actual = nu!(format!(
        r#"{prefix}{nu_bin} `--testbin` cococo foo/. foo/.. foo/... foo/./bar foo/../bar foo/.../bar ./bar ../bar .../bar"#
    ));

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

#[apply(run_external_prefixes)]
fn external_command_ndots_leading_dot_slash(nu_bin: &str, prefix: &str) {
    // Don't expand ndots with a leading `./`
    let actual = nu!(format!(
        r#"{prefix}{nu_bin} `--testbin` cococo ./... ./...."#
    ));

    assert_eq!(actual.out, "./... ./....");
}

#[apply(run_external_prefixes)]
fn external_command_url_args(nu_bin: &str, prefix: &str) {
    // If ndots is not handled correctly, we can lose the double forward slashes that are needed
    // here
    let actual = nu!(format!(
        r#"{prefix}{nu_bin} `--testbin` cococo http://example.com http://example.com/.../foo //foo"#
    ));

    assert_eq!(
        actual.out,
        "http://example.com http://example.com/.../foo //foo"
    );
}

#[apply(run_external_prefixes)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "only runs on Linux, where controlling the HOME var is reliable"
)]
fn external_command_expand_tilde(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
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
            format!(r#"{prefix}~/test_nu `--testbin` cococo hello"#)
        );
        assert_eq!(actual.out, "hello");
    })
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[rstest]
#[case("")]
#[case("^")]
fn external_arg_expand_tilde(nu_bin: &str, #[case] prefix: &str) {
    Playground::setup("external arg expand tilde", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}{nu_bin} `--testbin` cococo ~/foo ~/(2 + 2)"#)
        );

        let home = dirs::home_dir().expect("failed to find home dir");

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

#[apply(run_external_prefixes)]
fn external_command_not_expand_tilde_with_quotes(nu_bin: &str, prefix: &str) {
    Playground::setup(
        "external command not expand tilde with quotes",
        |dirs, _| {
            let actual = nu!(cwd: dirs.test(), format!(r#"{prefix}{nu_bin} `--testbin` nonu "~""#));
            assert_eq!(actual.out, r#"~"#);
        },
    )
}

#[apply(run_external_prefixes)]
fn external_command_expand_tilde_with_back_quotes(nu_bin: &str, prefix: &str) {
    Playground::setup(
        "external command not expand tilde with quotes",
        |dirs, _| {
            let actual = nu!(cwd: dirs.test(), format!(r#"{prefix}{nu_bin} `--testbin` nonu `~`"#));
            assert!(!actual.out.contains('~'));
        },
    )
}

#[apply(run_external_prefixes)]
fn external_command_receives_raw_binary_data(nu_bin: &str, prefix: &str) {
    Playground::setup("external command receives raw binary data", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!("0x[deadbeef] | {prefix}{nu_bin} `--testbin` input_bytes_length")
        );
        assert_eq!(actual.out, r#"4"#);
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_cmd_files(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows cmd file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.cmd",
            r#"
                @echo off
                echo Hello World
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), format!("{prefix}foo.cmd"));
        assert!(actual.out.contains("Hello World"));
    });
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_batch_files(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows batch file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.bat",
            r#"
                @echo off
                echo Hello World
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), format!("{prefix}foo.bat"));
        assert!(actual.out.contains("Hello World"));
    });
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_batch_files_without_cmd_extension(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup(
        "run a Windows cmd file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "foo.cmd",
                r#"
                @echo off
                echo Hello World
            "#,
            )]);

            let actual = nu!(cwd: dirs.test(), format!("{prefix}foo"));
            assert!(actual.out.contains("Hello World"));
        },
    );
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_batch_files_without_bat_extension(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
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

            let actual = nu!(cwd: dirs.test(), format!("{prefix}foo"));
            assert!(actual.out.contains("Hello World"));
        },
    );
}

#[apply(run_external_prefixes)]
fn quotes_trimmed_when_shelling_out(nu_bin: &str, prefix: &str) {
    // regression test for a bug where we weren't trimming quotes around string args before shelling out to cmd.exe
    let actual = nu!(format!(r#"{prefix}{nu_bin} `--testbin` cococo "foo""#));

    assert_eq!(actual.out, "foo");
}

#[cfg(not(windows))]
#[apply(run_external_prefixes)]
fn redirect_combine(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    Playground::setup("redirect_combine", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"{prefix}sh ...[-c 'echo Foo; echo >&2 Bar'] o+e>| print"#)
        );

        // Lines are collapsed in the nu! macro
        assert_eq!(actual.out, "FooBar");
    });
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_ps1_files(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run_a_windows_ps_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.ps1",
            r#"
                Write-Host Hello World
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), format!("{prefix}foo.ps1"));
        assert!(actual.out.contains("Hello World"));
    });
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
fn can_run_ps1_files_with_space_in_path(nu_bin: &str, prefix: &str) {
    let _ = nu_bin;
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run_a_windows_ps_file", |dirs, sandbox| {
        sandbox
            .within("path with space")
            .with_files(&[FileWithContent(
                "foo.ps1",
                r#"
                    Write-Host Hello World
                "#,
            )]);

        let actual = nu!(cwd: dirs.test().join("path with space"), format!("{prefix}foo.ps1"));
        assert!(actual.out.contains("Hello World"));
    });
}

#[rstest]
#[case("^")]
#[case("run-external ")]
fn can_run_external_without_path_env(nu_bin: &str, #[case] prefix: &str) {
    Playground::setup("can run external without path env", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"
                hide-env -i PATH
                hide-env -i Path
                {prefix}{nu_bin} `--testbin` cococo
            "#)
        );
        assert_eq!(actual.out, "cococo");
    })
}

#[rstest]
#[case("^")]
#[case("run-external ")]
fn expand_command_if_list(nu_bin: &str, #[case] prefix: &str) {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("expand command if list", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("foo.txt", "Hello World")]);
        let actual = nu!(
            cwd: dirs.test(),
            format!(r#"let cmd = ['{nu_bin}' `--testbin`]; {prefix}$cmd meow foo.txt"#),
        );

        assert!(actual.out.contains("Hello World"));
    })
}

#[rstest]
#[case("^")]
#[case("run-external ")]
fn error_when_command_list_empty(#[case] prefix: &str) {
    Playground::setup("error when command is list with no items", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), format!("let cmd = []; {prefix}$cmd"));
        assert!(actual.err.contains("missing parameter"));
    })
}
