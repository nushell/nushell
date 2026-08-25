use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn source_env_resolves_nested_source_relative_to_sourced_file(playground: Playground) -> Result {
    playground.file(
        "lib/my_library.nu",
        "source-env my_library/main.nu",
    )?;
    playground.file(
        "lib/my_library/main.nu",
        r#"$env.hello = "hello nu""#,
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("source-env lib/my_library.nu")?;
    tester.run("$env.hello").expect_value_eq("hello nu")
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
fn sources_unicode_file(
    #[ignore] playground: Playground,
    #[case] dir: &str,
    #[case] quote: &str,
) -> Result {
    let file = String::from_iter([dir, "/foo.nu"]);
    playground.dir(dir)?;
    playground.file(&file, "echo foo")?;

    let cmd = format!("source-env {quote}{file}{quote}");
    test()
        .cwd(playground.path())
        .run(&cmd)
        .expect_value_eq("foo")
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
    #[ignore] playground: Playground,
    #[case] dir: &str,
    #[case] quote: &str,
) -> Result {
    let file = String::from_iter([dir, "/foo.nu"]);
    playground.dir(dir)?;
    playground.file(&file, "echo foo")?;

    let cmd = format!("source-env {quote}{file}{quote}");
    test()
        .cwd(playground.path())
        .run(&cmd)
        .expect_value_eq("foo")
}

#[ignore]
#[test]
fn sources_unicode_file_in_non_utf8_dir() {
    // How do I create non-UTF-8 path???
}

#[ignore]
#[test]
fn can_source_dynamic_path(playground: Playground) -> Result {
    let foo_file = "foo.nu";
    playground.file(foo_file, "echo foo")?;

    let cmd = format!("let file = `{foo_file}`; source-env $file");
    test()
        .cwd(playground.path())
        .run(&cmd)
        .expect_value_eq("foo")
}

#[test]
fn source_env_eval_export_env(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        indoc::indoc! {"
        export-env { $env.FOO = 'foo' }
    "},
    )?;

    test()
        .cwd(playground.path())
        .run("source-env spam.nu; $env.FOO")
        .expect_value_eq("foo")
}

#[test]
fn source_env_eval_export_env_hide(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        indoc::indoc! {"
        export-env { hide-env FOO }
    "},
    )?;

    test()
        .cwd(playground.path())
        .run("$env.FOO = 'foo'; source-env spam.nu; $env.FOO")
        .expect_error_code_eq("nu::shell::column_not_found")
}

#[test]
fn source_env_do_cd(playground: Playground) -> Result {
    playground.file(
        "test1/test2/spam.nu",
        indoc::indoc! {"
            cd test1/test2
        "},
    )?;

    test()
        .cwd(playground.path())
        .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
        .expect_value_eq("test2")
}

#[test]
fn source_env_do_cd_file_relative(playground: Playground) -> Result {
    playground.file(
        "test1/test2/spam.nu",
        indoc::indoc! {"
            cd ($env.FILE_PWD | path join '..')
        "},
    )?;

    test()
        .cwd(playground.path())
        .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
        .expect_value_eq("test1")
}

#[test]
fn source_env_dont_cd_overlay(playground: Playground) -> Result {
    playground.file(
        "test1/test2/spam.nu",
        indoc::indoc! {"
            overlay new spam
            cd test1/test2
            overlay hide spam
        "},
    )?;

    test()
        .cwd(playground.path())
        .run("source-env test1/test2/spam.nu; $env.PWD | path basename")
        .expect_value_eq(playground.path().file_name().unwrap().to_string_lossy())
}

#[test]
fn source_env_is_scoped(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        indoc::indoc! {"
        def no-name-similar-to-this [] { 'no-name-similar-to-this' }
        alias nor-similar-to-this = echo 'nor-similar-to-this'
    "},
    )?;

    let err = test()
        .cwd(playground.path())
        .run("source-env spam.nu; no-name-similar-to-this")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::ExternalCommand { label, .. }
            if label == "Command `no-name-similar-to-this` not found"
    );

    let err = test()
        .cwd(playground.path())
        .run("source-env spam.nu; nor-similar-to-this")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::ExternalCommand { label, .. }
            if label == "Command `nor-similar-to-this` not found"
    );

    Ok(())
}

#[test]
fn source_env_const_file(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        indoc::indoc! {"
        $env.FOO = 'foo'
    "},
    )?;

    test()
        .cwd(playground.path())
        .run("const file = 'spam.nu'; source-env $file; $env.FOO")
        .expect_value_eq("foo")
}

#[test]
fn source_respects_early_return() -> Result {
    let _: Value = test()
        .cwd("tests/fixtures/formats")
        .run("source early_return.nu")?;
    Ok(())
}

#[test]
fn source_after_use_should_not_error(playground: Playground) -> Result {
    playground.empty_file("spam.nu")?;

    let () = test()
        .cwd(playground.path())
        .run("use spam.nu; source spam.nu")?;
    Ok(())
}

#[test]
fn use_after_source_should_not_error(playground: Playground) -> Result {
    playground.empty_file("spam.nu")?;

    let () = test()
        .cwd(playground.path())
        .run("source spam.nu; use spam.nu")?;
    Ok(())
}
