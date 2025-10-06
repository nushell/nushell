use nu_test_support::fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[should_panic]
#[test]
fn sources_also_files_under_custom_lib_dirs_path() {
    Playground::setup("source_test_1", |dirs, nu| {
        let file = dirs.test().join("config.toml");
        let library_path = dirs.test().join("lib");

        nu.with_config(file);
        nu.with_files(&[FileWithContent(
            "config.toml",
            &format!(
                r#"
                lib_dirs = ["{}"]
                skip_welcome_message = true
            "#,
                library_path.as_os_str().to_str().unwrap(),
            ),
        )]);

        nu.within("lib").with_files(&[FileWithContent(
            "my_library.nu",
            r#"
                source-env my_library/main.nu
            "#,
        )]);
        nu.within("lib/my_library").with_files(&[FileWithContent(
            "main.nu",
            r#"
                $env.hello = "hello nu"
            "#,
        )]);

        let actual = nu!("
            source-env my_library.nu ;
        
            hello
        ");

        assert_eq!(actual.out, "hello nu");
    })
}

fn try_source_foo_with_double_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(&[FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source-env ") + r#"""# + foo_file.as_str() + r#"""#;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

fn try_source_foo_with_single_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(&[FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source-env ") + r#"'"# + foo_file.as_str() + r#"'"#;

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

fn try_source_foo_without_quotes_in(testdir: &str, playdir: &str) {
    Playground::setup(playdir, |dirs, sandbox| {
        let testdir = String::from(testdir);
        let mut foo_file = testdir.clone();
        foo_file.push_str("/foo.nu");

        sandbox.mkdir(&testdir);
        sandbox.with_files(&[FileWithContent(&foo_file, "echo foo")]);

        let cmd = String::from("source-env ") + foo_file.as_str();

        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

#[test]
fn sources_unicode_file_in_normal_dir() {
    try_source_foo_with_single_quotes_in("foo", "source_test_1");
    try_source_foo_with_double_quotes_in("foo", "source_test_2");
    try_source_foo_without_quotes_in("foo", "source_test_3");
}

#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_1() {
    try_source_foo_with_single_quotes_in("🚒", "source_test_4");
    try_source_foo_with_double_quotes_in("🚒", "source_test_5");
    try_source_foo_without_quotes_in("🚒", "source_test_6");
}

#[cfg(not(windows))] // ':' is not allowed in Windows paths
#[test]
fn sources_unicode_file_in_unicode_dir_without_spaces_2() {
    try_source_foo_with_single_quotes_in(":fire_engine:", "source_test_7");
    try_source_foo_with_double_quotes_in(":fire_engine:", "source_test_8");
    try_source_foo_without_quotes_in(":fire_engine:", "source_test_9");
}

#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_1() {
    // this one fails
    try_source_foo_with_single_quotes_in("e-$ èрт🚒♞中片-j", "source_test_8");
    // this one passes
    try_source_foo_with_double_quotes_in("e-$ èрт🚒♞中片-j", "source_test_9");
}

#[cfg(not(windows))] // ':' is not allowed in Windows paths
#[test]
fn sources_unicode_file_in_unicode_dir_with_spaces_2() {
    try_source_foo_with_single_quotes_in("e-$ èрт:fire_engine:♞中片-j", "source_test_10");
    try_source_foo_with_double_quotes_in("e-$ èрт:fire_engine:♞中片-j", "source_test_11");
}

#[ignore]
#[test]
fn sources_unicode_file_in_non_utf8_dir() {
    // How do I create non-UTF-8 path???
}

#[ignore]
#[test]
fn can_source_dynamic_path() {
    Playground::setup("can_source_dynamic_path", |dirs, sandbox| {
        let foo_file = "foo.nu";

        sandbox.with_files(&[FileWithContent(foo_file, "echo foo")]);

        let cmd = format!("let file = `{foo_file}`; source-env $file");
        let actual = nu!(cwd: dirs.test(), &cmd);

        assert_eq!(actual.out, "foo");
    });
}

#[test]
fn source_env_eval_export_env() {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { $env.FOO = 'foo' }
            "#,
        )]);

        let inp = &[r#"source-env spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn source_env_eval_export_env_hide() {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { hide-env FOO }
            "#,
        )]);

        let inp = &[
            r#"$env.FOO = 'foo'"#,
            r#"source-env spam.nu"#,
            r#"$env.FOO"#,
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(actual.err.contains("not_found"));
    })
}

#[test]
fn source_env_do_cd() {
    Playground::setup("source_env_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    cd test1/test2
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn source_env_do_cd_file_relative() {
    Playground::setup("source_env_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    cd ($env.FILE_PWD | path join '..')
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn source_env_dont_cd_overlay() {
    Playground::setup("source_env_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    overlay new spam
                    cd test1/test2
                    overlay hide spam
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "source_env_dont_cd_overlay");
    })
}

#[test]
fn source_env_is_scoped() {
    Playground::setup("source_env_is_scoped", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                def no-name-similar-to-this [] { 'no-name-similar-to-this' }
                alias nor-similar-to-this = echo 'nor-similar-to-this'
            "#,
        )]);

        let inp = &[r#"source-env spam.nu"#, r#"no-name-similar-to-this"#];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(
            actual
                .err
                .contains("Command `no-name-similar-to-this` not found")
        );

        let inp = &[r#"source-env spam.nu"#, r#"nor-similar-to-this"#];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(
            actual
                .err
                .contains("Command `nor-similar-to-this` not found")
        );
    })
}

#[test]
fn source_env_const_file() {
    Playground::setup("source_env_const_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                $env.FOO = 'foo'
            "#,
        )]);

        let inp = &[
            r#"const file = 'spam.nu'"#,
            r#"source-env $file"#,
            r#"$env.FOO"#,
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn source_respects_early_return() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        source early_return.nu
    ");

    assert!(actual.err.is_empty());
}

#[test]
fn source_after_use_should_not_error() {
    Playground::setup("source_after_use", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.nu")]);

        let inp = &[r#"use spam.nu"#, r#"source spam.nu"#];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert!(actual.err.is_empty());
    })
}

#[test]
fn use_after_source_should_not_error() {
    Playground::setup("use_after_source", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.nu")]);

        let inp = &[r#"source spam.nu"#, r#"use spam.nu"#];
        let actual = nu!(cwd: dirs.test(), &inp.join("; "));
        assert!(actual.err.is_empty());
    })
}
