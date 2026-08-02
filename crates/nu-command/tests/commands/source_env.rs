use nu_test_support::fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::{Context, rstest};

#[test]
fn source_env_resolves_nested_source_relative_to_sourced_file() -> Result {
    Playground::setup("source_test_1", |dirs, nu| {
        nu.within("lib").with_files(&[FileWithContent(
            "my_library.nu",
            "
                source-env my_library/main.nu
            ",
        )]);
        nu.within("lib/my_library").with_files(&[FileWithContent(
            "main.nu",
            r#"
                $env.hello = "hello nu"
            "#,
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source-env lib/my_library.nu")?;
        tester.run("$env.hello").expect_value_eq("hello nu")
    })
}

#[rstest]
#[case::normal_dir_single_quotes("foo", "'")]
#[case::normal_dir_double_quotes("foo", "\"")]
#[case::normal_dir_without_quotes("foo", "")]
#[case::unicode_dir_single_quotes("🚒", "'")]
#[case::unicode_dir_double_quotes("🚒", "\"")]
#[case::unicode_dir_without_quotes("🚒", "")]
#[case::unicode_spaced_dir_single_quotes("e-$ èрт🚒♞中片-j", "'")]
#[case::unicode_spaced_dir_double_quotes("e-$ èрт🚒♞中片-j", "\"")]
fn sources_unicode_file(#[context] ctx: Context, #[case] dir: &str, #[case] quote: &str) -> Result {
    Playground::setup(ctx.description.unwrap(), |dirs, sandbox| {
        let file = String::from_iter([dir, "/foo.nu"]);
        sandbox.mkdir(dir);
        sandbox.with_files(&[FileWithContent(&file, "echo foo")]);

        let cmd = format!("source-env {quote}{file}{quote}");
        test().cwd(dirs.test()).run(&cmd).expect_value_eq("foo")
    })
}

#[cfg(not(windows))] // ':' is not allowed in Windows paths
#[rstest]
#[case::colon_dir_single_quotes(":fire_engine:", "'")]
#[case::colon_dir_double_quotes(":fire_engine:", "\"")]
#[case::colon_dir_without_quotes(":fire_engine:", "")]
#[case::colon_spaced_dir_single_quotes("e-$ èрт:fire_engine:♞中片-j", "'")]
#[case::colon_spaced_dir_double_quotes("e-$ èрт:fire_engine:♞中片-j", "\"")]
#[nu_test_support::test]
fn sources_unicode_file_in_colon_dir(
    #[context] ctx: Context,
    #[case] dir: &str,
    #[case] quote: &str,
) -> Result {
    Playground::setup(ctx.description.unwrap(), |dirs, sandbox| {
        let file = String::from_iter([dir, "/foo.nu"]);
        sandbox.mkdir(&dir);
        sandbox.with_files(&[FileWithContent(&file, "echo foo")]);

        let cmd = format!("source-env {quote}{file}{quote}");
        test().cwd(dirs.test()).run(&cmd).expect_value_eq("foo")
    })
}

#[ignore]
#[test]
fn sources_unicode_file_in_non_utf8_dir() {
    // How do I create non-UTF-8 path???
}

#[ignore]
#[test]
fn can_source_dynamic_path() -> Result {
    Playground::setup("can_source_dynamic_path", |dirs, sandbox| {
        let foo_file = "foo.nu";
        sandbox.with_files(&[FileWithContent(foo_file, "echo foo")]);

        let cmd = format!("let file = `{foo_file}`; source-env $file");
        test().cwd(dirs.test()).run(&cmd).expect_value_eq("foo")
    })
}

#[test]
fn source_env_eval_export_env() -> Result {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                export-env { $env.FOO = 'foo' }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("source-env spam.nu; $env.FOO")
            .expect_value_eq("foo")
    })
}

#[test]
fn source_env_eval_export_env_hide() -> Result {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                export-env { hide-env FOO }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("$env.FOO = 'foo'; source-env spam.nu; $env.FOO")
            .expect_error_code_eq("nu::shell::column_not_found")
    })
}

#[test]
fn source_env_do_cd() -> Result {
    Playground::setup("source_env_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    cd test1/test2
                ",
            )]);

        test()
            .cwd(dirs.test())
            .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
            .expect_value_eq("test2")
    })
}

#[test]
fn source_env_do_cd_file_relative() -> Result {
    Playground::setup("source_env_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    cd ($env.FILE_PWD | path join '..')
                ",
            )]);

        test()
            .cwd(dirs.test())
            .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
            .expect_value_eq("test1")
    })
}

#[test]
fn source_env_dont_cd_overlay() -> Result {
    Playground::setup("source_env_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    overlay new spam
                    cd test1/test2
                    overlay hide spam
                ",
            )]);

        test()
            .cwd(dirs.test())
            .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
            .expect_value_eq("source_env_dont_cd_overlay")
    })
}

#[test]
fn source_env_is_scoped() -> Result {
    Playground::setup("source_env_is_scoped", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                def no-name-similar-to-this [] { 'no-name-similar-to-this' }
                alias nor-similar-to-this = echo 'nor-similar-to-this'
            ",
        )]);

        let err = test()
            .cwd(dirs.test())
            .run("source-env spam.nu; no-name-similar-to-this")
            .expect_shell_error()?;
        assert_matches!(
            err,
            ShellError::ExternalCommand { label, .. }
                if label == "Command `no-name-similar-to-this` not found"
        );

        let err = test()
            .cwd(dirs.test())
            .run("source-env spam.nu; nor-similar-to-this")
            .expect_shell_error()?;
        assert_matches!(
            err,
            ShellError::ExternalCommand { label, .. }
                if label == "Command `nor-similar-to-this` not found"
        );

        Ok(())
    })
}

#[test]
fn source_env_const_file() -> Result {
    Playground::setup("source_env_const_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                $env.FOO = 'foo'
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("const file = 'spam.nu'; source-env $file; $env.FOO")
            .expect_value_eq("foo")
    })
}

#[test]
fn source_respects_early_return() -> Result {
    let _: Value = test()
        .cwd("tests/fixtures/formats")
        .run("source early_return.nu")?;
    Ok(())
}

#[test]
fn source_after_use_should_not_error() -> Result {
    Playground::setup("source_after_use", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.nu")]);

        let () = test().cwd(dirs.test()).run("use spam.nu; source spam.nu")?;
        Ok(())
    })
}

#[test]
fn use_after_source_should_not_error() -> Result {
    Playground::setup("use_after_source", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.nu")]);

        let () = test().cwd(dirs.test()).run("source spam.nu; use spam.nu")?;
        Ok(())
    })
}
