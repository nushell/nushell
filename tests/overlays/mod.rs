use itertools::Itertools;
use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::add_overlay([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use spam",
    "foo",
], "foo")]
#[case::add_overlay_as_new_name([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use spam as spam_new",
    "foo",
], "foo")]
#[case::add_overlay_twice([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use spam",
    "overlay use spam",
    "foo",
], "foo")]
#[case::add_prefixed_overlay([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use --prefix spam",
    "spam foo",
], "foo")]
#[case::add_prefixed_overlay_twice([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use --prefix spam",
    "overlay use --prefix spam",
    "spam foo",
], "foo")]
fn overlay_use_success<const C: usize>(
    #[case] commands: [&str; C],
    #[case] expected: impl IntoValue + Clone,
) -> Result {
    test()
        .run(commands.iter().join("; "))
        .expect_value_eq(expected.clone())?;
    test().run_multiple(commands).expect_value_eq(expected)?;
    Ok(())
}

#[rstest]
#[case::prefixed_then_unprefixed([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use --prefix spam",
    "overlay use spam",
])]
#[case::unprefixed_then_prefixed([
    r#"module spam { export def foo [] { "foo" } }"#,
    "overlay use spam",
    "overlay use --prefix spam",
])]
fn overlay_prefix_mismatch<const C: usize>(#[case] commands: [&str; C]) -> Result {
    test()
        .run(commands.iter().join("; "))
        .expect_parse_error()?;
    test()
        .run_multiple(commands)
        .expect_error_code_eq("nu::parser::overlay_prefix_mismatch")?;
    Ok(())
}

#[test]
fn prefixed_overlay_keeps_custom_decl() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use --prefix spam",
        r#"def bar [] { "bar" }"#,
        "overlay hide --keep-custom spam",
        "bar",
    ];

    test().run(commands.join("; ")).expect_value_eq("bar")?;
    test().run_multiple(commands).expect_value_eq("bar")?;
    Ok(())
}

#[test]
fn def_before_overlay_use_should_work() -> Result {
    let commands = [
        r#"def something [] { "example" }"#,
        "module spam { }",
        "overlay use spam",
        r#"def bar [] { "bar" }"#,
        "overlay hide spam",
        "bar",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    let error = test()
        .run_multiple::<Value>(commands)
        .expect_shell_error()?;
    assert_contains("Command `bar` not found", format!("{error:?}"));
    Ok(())
}

#[test]
fn define_module_before_overlay_inside_func_should_work() -> Result {
    let commands = [
        r#"
            def main [] {
                module spam { export def foo [] { "foo" } }
                overlay use spam
                def bar [] { "bar" }
                overlay hide spam
                bar # Returns bar
            };
        "#,
        "main",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    Ok(())
}

#[test]
fn add_overlay_env() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn add_prefixed_overlay_env_no_prefix() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use --prefix spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[rstest]
#[case::decl(["overlay use samples/spam.nu", "foo"], "foo")]
#[case::alias(["overlay use samples/spam.nu", "bar"], "bar")]
#[case::env(["overlay use samples/spam.nu", "$env.BAZ"], "baz")]
fn overlay_use_from_file<const C: usize>(
    #[case] commands: [&str; C],
    #[case] expected: impl IntoValue + Clone,
) -> Result {
    test()
        .cwd("tests/overlays")
        .run(commands.iter().join("; "))
        .expect_value_eq(expected.clone())?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq(expected)?;
    Ok(())
}

#[test]
fn add_overlay_from_const_file_decl() -> Result {
    let commands = ["const file = 'samples/spam.nu'", "overlay use $file", "foo"];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn add_overlay_from_const_module_name_decl() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "const mod = 'spam'",
        "overlay use $mod",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn add_overlay_from_file_with_stored_where_condition() -> Result {
    Playground::setup(
        "add_overlay_from_file_with_stored_where_condition",
        |dirs, sandbox| -> Result {
            sandbox.with_files(&[FileWithContentToBeTrimmed(
                "mod.nu",
                r#"
                export def helper [] {
                    let cond = {|x| true }
                    [{a: 1}] | where $cond
                }

                export def main [] { "ok" }
            "#,
            )]);

            let commands = ["overlay use mod.nu", "helper | to nuon --raw"];

            test()
                .cwd(dirs.test())
                .run(commands.join("; "))
                .expect_value_eq("[[a];[1]]")?;
            test()
                .cwd(dirs.test())
                .run_multiple(commands)
                .expect_value_eq("[[a];[1]]")
        },
    )
}

#[test]
fn new_overlay_from_const_name() -> Result {
    let commands = [
        "const mod = 'spam'",
        "overlay new $mod",
        "overlay list | last | get name",
    ];

    test().run_multiple(commands).expect_value_eq("spam")
}

#[test]
fn hide_overlay_from_const_name() -> Result {
    let commands = [
        "const mod = 'spam'",
        "overlay new $mod",
        "overlay hide $mod",
        "overlay list | where active == true | get name | str join ' '",
    ];

    let out: String = test().run_multiple(commands)?;
    assert_contains_not("spam", out);
    Ok(())
}

// This one tests that separate pipeline execution updates the working directory
#[test]
fn add_overlay_from_file_decl_cd() -> Result {
    let mut tester = test().cwd("tests/overlays");
    let () = tester.run("cd samples")?;
    let cwd: String = tester.run("$env.PWD")?;
    tester = tester.cwd(cwd);
    let () = tester.run("overlay use spam.nu")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn add_overlay_scoped() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "do { overlay use spam }",
        "foo",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn update_overlay_from_module() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        r#"module spam { export def foo [] { "bar" } }"#,
        "overlay use spam",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("bar")?;
    test().run_multiple(commands).expect_value_eq("bar")?;
    Ok(())
}

#[test]
fn update_overlay_from_module_env() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        r#"module spam { export-env { $env.FOO = "bar" } }"#,
        "overlay use spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("bar")?;
    test().run_multiple(commands).expect_value_eq("bar")?;
    Ok(())
}

#[test]
fn overlay_use_do_not_eval_twice() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        r#"$env.FOO = "bar""#,
        "overlay hide spam",
        "overlay use spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("bar")?;
    test().run_multiple(commands).expect_value_eq("bar")?;
    Ok(())
}

#[test]
fn hide_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "foo",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn hide_last_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide",
        "foo",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn hide_overlay_scoped() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "do { overlay hide spam }",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn hide_overlay_env() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn hide_overlay_scoped_env() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "do { overlay hide spam }",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn list_default_overlay() -> Result {
    let commands = ["overlay list | last | get name"];

    test().run(commands.join("; ")).expect_value_eq("zero")?;
    test().run_multiple(commands).expect_value_eq("zero")?;
    Ok(())
}

#[test]
fn list_last_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay list | last | get name",
    ];

    test().run(commands.join("; ")).expect_value_eq("spam")?;
    test().run_multiple(commands).expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn list_overlay_scoped() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "do { overlay list | last | get name }",
    ];

    test().run(commands.join("; ")).expect_value_eq("spam")?;
    test().run_multiple(commands).expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn hide_overlay_discard_decl() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        r#"def bagr [] { "bagr" }"#,
        "overlay hide spam",
        "bagr",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_discard_alias() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        r#"alias bagr = echo "bagr""#,
        "overlay hide spam",
        "bagr",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_discard_env() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "overlay hide spam",
        "$env.BAGR",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_keep_decl() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        r#"def bagr [] { "bagr" }"#,
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("bagr")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("bagr")?;
    Ok(())
}

#[test]
fn hide_overlay_keep_alias() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "alias bagr = echo 'bagr'",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("bagr")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("bagr")?;
    Ok(())
}

#[test]
fn hide_overlay_dont_keep_env() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "overlay hide --keep-custom spam",
        "$env.BAGR",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_dont_keep_overwritten_decl() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "def foo [] { 'bar' }",
        "overlay hide --keep-custom spam",
        "foo",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_dont_keep_overwritten_alias() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "alias bar = echo `baz`",
        "overlay hide --keep-custom spam",
        "bar",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_dont_keep_overwritten_env() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "$env.BAZ = 'bagr'",
        "overlay hide --keep-custom spam",
        "$env.BAZ",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn hide_overlay_keep_decl_in_latest_overlay() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "def bagr [] { 'bagr' }",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("bagr")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("bagr")?;
    Ok(())
}

#[test]
fn hide_overlay_keep_alias_in_latest_overlay() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "alias bagr = echo 'bagr'",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "bagr",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("bagr")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("bagr")?;
    Ok(())
}

#[test]
fn hide_overlay_dont_keep_env_in_latest_overlay() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        "$env.BAGR = 'bagr'",
        "module eggs { }",
        "overlay use eggs",
        "overlay hide --keep-custom spam",
        "$env.BAGR",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test().run_multiple(commands).expect_parse_error()?;
    Ok(())
}

#[test]
fn preserve_overrides() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        r#"def foo [] { "new-foo" }"#,
        "overlay hide spam",
        "overlay use spam",
        "foo",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("new-foo")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("new-foo")?;
    Ok(())
}

#[test]
fn reset_overrides() -> Result {
    let commands = [
        "overlay use samples/spam.nu",
        r#"def foo [] { "new-foo" }"#,
        "overlay hide spam",
        "overlay use samples/spam.nu",
        "foo",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("foo")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_new() -> Result {
    let commands = ["overlay new spam", "overlay list | last | get name"];

    test().run(commands.join("; ")).expect_value_eq("spam")?;
    test().run_multiple(commands).expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_keep_pwd() -> Result {
    let commands = [
        "overlay new spam",
        "cd samples",
        "overlay hide --keep-env [ PWD ] spam",
        "$env.PWD | path basename",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("samples")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("samples")?;
    Ok(())
}

#[test]
fn overlay_reactivate_with_nufile_should_not_change_pwd() -> Result {
    let commands = [
        "overlay use spam.nu",
        "cd ..",
        "overlay hide --keep-env [ PWD ] spam",
        "cd samples",
        "overlay use spam.nu",
        "$env.PWD | path basename",
    ];

    test()
        .cwd("tests/overlays/samples")
        .run(commands.join("; "))
        .expect_value_eq("samples")?;
    test()
        .cwd("tests/overlays/samples")
        .run_multiple(commands)
        .expect_value_eq("samples")?;
    Ok(())
}

#[test]
fn overlay_reactivate_with_module_name_keeps_pwd() -> Result {
    let commands = [
        "overlay use spam.nu",
        "cd ..",
        "overlay hide spam",
        "cd samples",
        "overlay use spam",
        "$env.PWD | path basename",
    ];

    test()
        .cwd("tests/overlays/samples")
        .run(commands.join("; "))
        .expect_value_eq("samples")?;
    test()
        .cwd("tests/overlays/samples")
        .run_multiple(commands)
        .expect_value_eq("samples")?;
    Ok(())
}

#[test]
fn overlay_new_reactivating_keeps_pwd() -> Result {
    let commands = [
        "overlay new spam",
        "cd samples",
        "overlay hide spam",
        "cd ..",
        "overlay new spam",
        "$env.PWD | path basename",
    ];

    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("overlays")
}

// `OLDPWD` has to follow `PWD`, or `cd -` goes somewhere you weren't just in.
#[rstest]
#[case::hide(&["overlay new spam", "cd samples", "overlay hide spam"], "overlays")]
#[case::use_by_name(
    &[
        "overlay use samples/spam.nu",
        "cd samples",
        "overlay hide spam",
        "cd ..",
        "cd samples",
        "cd ..",
        "overlay use spam",
    ],
    "samples"
)]
#[case::new(
    &[
        "overlay new spam",
        "cd samples",
        "overlay hide spam",
        "cd ..",
        "cd samples",
        "cd ..",
        "overlay new spam",
    ],
    "samples"
)]
fn overlay_changes_keep_oldpwd(#[case] commands: &[&str], #[case] expected: &str) -> Result {
    let mut tester = test().cwd("tests/overlays");
    for command in commands {
        let () = tester.run(*command)?;
    }
    tester
        .run("$env.OLDPWD | path basename")
        .expect_value_eq(expected)
}

// An `export-env` block can still move you when an overlay comes back.
#[test]
fn overlay_reload_keeps_cd_from_export_env() -> Result {
    Playground::setup("overlay_reload_export_env_cd", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("sub")
            .with_files(&[FileWithContent("spam.nu", "export-env { cd sub }")]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run("overlay hide spam")?;
        let () = tester.run("cd ..")?;
        let () = tester.run("overlay use --reload spam")?;
        tester
            .run("$env.PWD | path basename")
            .expect_value_eq("sub")
    })
}

// The overlay's name isn't bound to a module in these, so only its origin can find the file.
#[rstest]
#[case::renamed(
    "overlay_reload_renamed",
    "foo.nu",
    &["use foo.nu", "overlay use foo.nu as bar", "overlay hide bar"],
    "overlay use bar"
)]
#[case::imported_in_another_overlay(
    "overlay_reload_other_overlay",
    "spam.nu",
    &["overlay new tmp", "overlay use spam.nu", "overlay hide tmp"],
    "overlay use spam"
)]
fn overlay_by_name_reads_a_changed_file_without_a_module_name(
    #[case] topic: &str,
    #[case] file: &str,
    #[case] first: &[&str],
    #[case] again: &str,
) -> Result {
    Playground::setup(topic, |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(file, "export def v [] { 1 }")]);

        let mut tester = test().cwd(dirs.test());
        for command in first {
            let () = tester.run(*command)?;
        }
        let () = tester.run_with_data(format!("save -f {file}"), "export def v [] { 2 }")?;
        let () = tester.run(again)?;
        tester.run("v").expect_value_eq(2)
    })
}

// A bare overlay name isn't a path, so `FILE_PWD` has to come from the module's file.
#[test]
fn overlay_by_name_reload_sets_file_pwd() -> Result {
    Playground::setup("overlay_reload_file_pwd", |dirs, sandbox| -> Result {
        let code = "export-env { $env.WHERE = ($env.FILE_PWD | path basename) }";
        sandbox
            .mkdir("mods")
            .with_files(&[FileWithContent("mods/foo.nu", code)]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use mods/foo.nu")?;
        let () = tester.run("overlay hide foo")?;
        let () = tester.run_with_data("save -f mods/foo.nu", format!("{code}\n"))?;
        let () = tester.run("overlay use foo")?;
        tester.run("$env.WHERE").expect_value_eq("mods")
    })
}

#[test]
fn overlay_wrong_rename_type() -> Result {
    let commands = ["module spam {}", "overlay use spam as { echo foo }"];

    test().run(commands.join("; ")).expect_parse_error()?;
    Ok(())
}

#[test]
fn overlay_add_renamed() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs --prefix",
        "eggs foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_add_renamed_const() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "const name = 'spam'",
        "const new_name = 'eggs'",
        "overlay use $name as $new_name --prefix",
        "eggs foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_add_renamed_from_file() -> Result {
    let commands = ["overlay use samples/spam.nu as eggs --prefix", "eggs foo"];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("foo")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_cant_rename_existing_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay hide spam",
        "overlay use spam as eggs",
    ];

    test().run(commands.join("; ")).expect_parse_error()?;
    test()
        .run_multiple(commands)
        .expect_error_code_eq("nu::parser::cant_add_overlay_help")?;
    Ok(())
}

#[test]
fn overlay_can_add_renamed_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs --prefix",
        "overlay use spam --prefix",
        "(spam foo) + (eggs foo)",
    ];

    test().run(commands.join("; ")).expect_value_eq("foofoo")?;
    test().run_multiple(commands).expect_value_eq("foofoo")?;
    Ok(())
}

#[test]
fn overlay_hide_renamed_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo-command-which-does-not-conflict [] { "foo" } }"#,
        "overlay use spam as eggs",
        "overlay hide eggs",
        "foo-command-which-does-not-conflict",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    test()
        .run_multiple(commands)
        .expect_error_code_eq("nu::shell::external_command")?;
    Ok(())
}

#[test]
fn overlay_hide_restore_hidden_env() -> Result {
    let mut tester = test().env("foo", "bar");
    let () = tester.run("overlay new aa")?;
    let () = tester.run("hide-env foo")?;
    let () = tester.run("overlay hide aa")?;
    tester.run("$env.foo").expect_value_eq("bar")
}

#[test]
fn overlay_hide_dont_restore_hidden_env_which_is_introduce_currently() -> Result {
    let commands = [
        "overlay new aa",
        "$env.foo = 'bar'",
        "hide-env foo", // hide the env in overlay `aa`
        "overlay hide aa",
        "'foo' in $env",
    ];

    test().run_multiple(commands).expect_value_eq(false)?;
    Ok(())
}

#[test]
fn overlay_hide_and_add_renamed_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs",
        "overlay hide eggs",
        "overlay use eggs",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_use_export_env() -> Result {
    let commands = [
        "module spam { export-env { $env.FOO = 'foo' } }",
        "overlay use spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_use_export_env_config_affected() -> Result {
    let commands = [
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "$out | to json --raw",
    ];

    let Value::String { val, .. } = test().run(commands.join("; "))? else {
        panic!("expected string value")
    };
    assert_eq!(val.replace(",0 ", ".0 "), r#"["20.0 MB","20.0 MiB"]"#);
    let Value::String { val, .. } = test().run_multiple(commands)? else {
        panic!("expected string value")
    };
    assert_eq!(val.replace(",0 ", ".0 "), r#"["20.0 MB","20.0 MiB"]"#);
    Ok(())
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn overlay_hide_config_affected() -> Result {
    let commands = [
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "overlay hide",
        "$out ++= [(20MB | into string)]",
        "$out",
    ];

    // Can't hide overlay within the same source file.
    test()
        .run_multiple(commands)
        .expect_value_eq(["20.0 MB", "20.0 MiB", "20.0 MB"])
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.UTF-8")]
fn overlay_use_after_hide_config_affected() -> Result {
    let commands = [
        "mut out = []",
        "$env.config.filesize.unit = 'metric'",
        "$out ++= [(20MB | into string)]",
        "module spam { export-env { $env.config.filesize.unit = 'binary' } }",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "overlay hide",
        "$out ++= [(20MB | into string)]",
        "overlay use spam",
        "$out ++= [(20MiB | into string)]",
        "$out",
    ];

    // Can't hide overlay within the same source file.
    test()
        .run_multiple(commands)
        .expect_value_eq(["20.0 MB", "20.0 MiB", "20.0 MB", "20.0 MiB"])
}

#[test]
fn overlay_use_export_env_hide() -> Result {
    let commands = [
        "$env.FOO = 'foo'",
        "module spam { export-env { hide-env FOO } }",
        "overlay use spam",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn overlay_use_do_cd() -> Result {
    Playground::setup("overlay_use_do_cd", |dirs, sandbox| -> Result {
        sandbox.mkdir("test1/test2").with_files(&[FileWithContent(
            "test1/test2/spam.nu",
            "export-env { cd test1/test2 }",
        )]);

        let code = "
            overlay use test1/test2/spam.nu
            $env.PWD | path basename
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq("test2")
    })
}

#[test]
fn overlay_use_do_cd_file_relative() -> Result {
    Playground::setup(
        "overlay_use_do_cd_file_relative",
        |dirs, sandbox| -> Result {
            sandbox.mkdir("test1/test2").with_files(&[FileWithContent(
                "test1/test2/spam.nu",
                "export-env { cd ($env.FILE_PWD | path join '..') }",
            )]);

            let code = "
                overlay use test1/test2/spam.nu
                $env.PWD | path basename
            ";

            test().cwd(dirs.test()).run(code).expect_value_eq("test1")
        },
    )
}

#[test]
fn overlay_hide_in_export_env_keeps_cd() -> Result {
    Playground::setup(
        "overlay_hide_in_export_env_keeps_cd",
        |dirs, sandbox| -> Result {
            sandbox
                .mkdir("test1/test2")
                .with_files(&[FileWithContentToBeTrimmed(
                    "test1/test2/spam.nu",
                    "
                    export-env {
                        overlay new spam
                        cd test1/test2
                        overlay hide spam
                    }
                ",
                )]);

            let code = "
           source-env test1/test2/spam.nu
           $env.PWD | path basename
        ";

            test().cwd(dirs.test()).run(code).expect_value_eq("test2")
        },
    )
}

#[test]
fn overlay_use_find_scoped_module() -> Result {
    Playground::setup("overlay_use_find_module_scoped", |dirs, _| -> Result {
        let code = "
                do {
                    module spam { }

                    overlay use spam
                    overlay list | last | get name
                }
            ";

        test().cwd(dirs.test()).run(code).expect_value_eq("spam")
    })
}

#[test]
fn overlay_preserve_hidden_env_1() -> Result {
    let commands = [
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "overlay use eggs",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_preserve_hidden_env_2() -> Result {
    let commands = [
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay hide spam",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "overlay hide eggs",
        "overlay use spam",
        "overlay use eggs",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_reset_hidden_env() -> Result {
    let commands = [
        "overlay new spam",
        "$env.FOO = 'foo'",
        "overlay new eggs",
        "$env.FOO = 'bar'",
        "hide-env FOO",
        "module eggs { export-env { $env.FOO = 'bar' } }",
        "overlay use eggs",
        "$env.FOO",
    ];

    test().run(commands.join("; ")).expect_value_eq("bar")?;
    test().run_multiple(commands).expect_value_eq("bar")?;
    Ok(())
}

#[ignore = "TODO: For this to work, we'd need to make predecls respect overlays"]
#[test]
fn overlay_preserve_hidden_decl() -> Result {
    let commands = [
        "overlay new spam",
        "def foo [] { 'foo' }",
        "overlay new eggs",
        "def foo [] { 'bar' }",
        "hide foo",
        "overlay use eggs",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[ignore = "TODO: For this to work, we'd need to make predecls respect overlays"]
#[test]
fn overlay_preserve_hidden_alias() -> Result {
    let commands = [
        "overlay new spam",
        "alias foo = echo 'foo'",
        "overlay new eggs",
        "alias foo = echo 'bar'",
        "hide foo",
        "overlay use eggs",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_trim_single_quote() -> Result {
    let code = r#"
        module spam { export def foo [] { "foo" } }
        overlay use 'spam'
        overlay list | last | get name
    "#;

    test().run(code).expect_value_eq("spam")
}

#[test]
fn overlay_trim_single_quote_hide() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use 'spam'",
        "overlay hide spam ",
        "foo",
    ];
    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn overlay_trim_double_quote() -> Result {
    let code = r#"
        module spam { export def foo [] { "foo" } }
        overlay use "spam"
        overlay list | last | get name
    "#;

    test().run(code).expect_value_eq("spam")
}

#[test]
fn overlay_trim_double_quote_hide() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        "overlay hide spam ",
        "foo",
    ];
    test().run(commands.join("; ")).expect_shell_error()?;
    test().run_multiple(commands).expect_shell_error()?;
    Ok(())
}

#[test]
fn overlay_use_and_restore_older_env_vars() -> Result {
    let commands = [
        "module spam {
            export-env {
                let old_baz = $env.BAZ;
                $env.BAZ = $old_baz + 'baz'
            }
        }",
        "$env.BAZ = 'baz'",
        "overlay use spam",
        "overlay hide spam",
        "$env.BAZ = 'new-baz'",
        "overlay use --reload spam",
        "$env.BAZ",
    ];

    test()
        .run(commands.join("; "))
        .expect_value_eq("new-bazbaz")?;
    test()
        .run_multiple(commands)
        .expect_value_eq("new-bazbaz")?;
    Ok(())
}

#[test]
fn overlay_use_and_reload() -> Result {
    let commands = [
        "module spam {
            export def foo [] { 'foo' };
            export alias fooalias = echo 'foo';
            export-env {
                $env.FOO = 'foo'
            }
        }",
        "overlay use spam",
        "def foo [] { 'newfoo' }",
        "alias fooalias = echo 'newfoo'",
        "$env.FOO = 'newfoo'",
        "overlay use --reload spam",
        "$'(foo)(fooalias)($env.FOO)'",
    ];

    test()
        .run(commands.join("; "))
        .expect_value_eq("foofoofoo")?;
    test().run_multiple(commands).expect_value_eq("foofoofoo")?;
    Ok(())
}

#[test]
fn overlay_use_and_reolad_keep_custom() -> Result {
    let commands = [
        "overlay new spam",
        "def foo [] { 'newfoo' }",
        "alias fooalias = echo 'newfoo'",
        "$env.FOO = 'newfoo'",
        "overlay use --reload spam",
        "$'(foo)(fooalias)($env.FOO)'",
    ];

    test()
        .run(commands.join("; "))
        .expect_value_eq("newfoonewfoonewfoo")?;
    test()
        .run_multiple(commands)
        .expect_value_eq("newfoonewfoonewfoo")?;
    Ok(())
}

#[test]
fn overlay_use_main() -> Result {
    let code = r#"
        module spam { export def main [] { "spam" } }
        overlay use spam
        spam
    "#;

    test().run(code).expect_value_eq("spam")
}

#[test]
fn overlay_use_main_prefix() -> Result {
    let code = r#"
        module spam { export def main [] { "spam" } }
        overlay use spam --prefix
        spam
    "#;

    test().run(code).expect_value_eq("spam")
}

#[test]
fn overlay_use_main_def_env() -> Result {
    let code = r#"
        module spam { export def --env main [] { $env.SPAM = "spam" } }
        overlay use spam
        spam
        $env.SPAM
    "#;

    test().run(code).expect_value_eq("spam")
}

#[test]
fn overlay_use_main_def_known_external() -> Result {
    // note: requires installed cargo
    let commands = [
        "module cargo { export extern main [] }",
        "overlay use cargo",
        "cargo --version",
    ];

    let out: String = test().inherit_rust_toolchain_env().run_multiple(commands)?;
    assert_contains("cargo", out);
    Ok(())
}

#[test]
fn overlay_use_main_not_exported() -> Result {
    let code = r#"
        module my-super-cool-and-unique-module-name { def main [] { "hi" } }
        overlay use my-super-cool-and-unique-module-name
        my-super-cool-and-unique-module-name
    "#;

    test().run(code).expect_shell_error()?;
    Ok(())
}

#[test]
fn alias_overlay_hide() -> Result {
    let commands = [
        "overlay new spam",
        "def my-epic-command-name [] { 'foo' }",
        "overlay new eggs",
        "alias oh = overlay hide",
        "oh spam",
        "my-epic-command-name",
    ];

    test()
        .run(commands.join("; "))
        .expect_error_code_eq("nu::shell::external_command")?;
    test()
        .run_multiple(commands)
        .expect_error_code_eq("nu::shell::external_command")?;
    Ok(())
}

#[test]
fn alias_overlay_use() -> Result {
    let commands = [
        "module spam { export def foo [] { 'foo' } }",
        "alias ou = overlay use",
        "ou spam",
        "foo",
    ];

    test().run(commands.join("; ")).expect_value_eq("foo")?;
    test().run_multiple(commands).expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn alias_overlay_use_2() -> Result {
    let commands = [
        "module inner {}",
        "module spam { export alias b = overlay use inner }",
        "use spam",
        "spam b",
        "overlay list | get 1.name",
    ];

    test().run(commands.join("; ")).expect_value_eq("inner")?;
    test().run_multiple(commands).expect_value_eq("inner")?;
    Ok(())
}

#[test]
fn alias_overlay_use_3() -> Result {
    let commands = [
        "module inner {}",
        "module spam { export alias b = overlay use inner }",
        "use spam b",
        "b",
        "overlay list | get 1.name",
    ];

    test().run(commands.join("; ")).expect_value_eq("inner")?;
    test().run_multiple(commands).expect_value_eq("inner")?;
    Ok(())
}

#[test]
fn alias_overlay_new() -> Result {
    let commands = [
        "alias on = overlay new",
        "on spam",
        "on eggs",
        "overlay list | last | get name",
    ];

    test().run(commands.join("; ")).expect_value_eq("eggs")?;
    test().run_multiple(commands).expect_value_eq("eggs")?;
    Ok(())
}

#[test]
fn overlay_new_with_reload() -> Result {
    let commands = [
        "overlay new spam",
        "$env.foo = 'bar'",
        "overlay hide spam",
        "overlay new spam -r",
        "'foo' in $env",
    ];

    test().run(commands.join("; ")).expect_value_eq(false)?;
    test().run_multiple(commands).expect_value_eq(false)?;
    Ok(())
}

#[rstest]
#[case::main("spam", "spam")]
#[case::foo("foo", "foo")]
#[case::bar("bar", "bar")]
#[case::foo_baz("foo baz", "foobaz")]
#[case::bar_baz("bar baz", "barbaz")]
#[case::baz("baz", "spambaz")]
fn overlay_use_module_dir(#[case] code: &str, #[case] expected: impl IntoValue) -> Result {
    let commands = ["overlay use samples/spam", code];
    test()
        .cwd("tests/modules")
        .run(commands.iter().join("; "))
        .expect_value_eq(expected)
}

#[test]
fn overlay_use_module_dir_with_dot_by_path() -> Result {
    Playground::setup(
        "overlay_use_module_dir_with_dot",
        |dirs, sandbox| -> Result {
            sandbox.mkdir("spam.eggs").with_files(&[FileWithContent(
                "spam.eggs/mod.nu",
                r#"export def foo [] { "foo" }"#,
            )]);

            let mut tester = test().cwd(dirs.test());
            let () = tester.run("overlay use ./spam.eggs")?;
            tester
                .run("overlay list | where active | get name | where $it =~ spam | to nuon")
                .expect_value_eq(r#"["spam.eggs"]"#)?;
            let () = tester.run("overlay hide spam.eggs")?;
            tester
                .run("overlay list | where active | get name | where $it =~ spam | length")
                .expect_value_eq(0)
        },
    )
}

#[rstest]
#[case("spam", "spam")]
#[case("spam foo", "foo")]
#[case("spam bar", "bar")]
#[case("spam foo baz", "foobaz")]
#[case("spam bar baz", "barbaz")]
#[case("spam baz", "spambaz")]
fn overlay_use_module_dir_prefix(#[case] code: &str, #[case] expected: impl IntoValue) -> Result {
    let mut tester = test().cwd("tests/modules");
    let () = tester.run("overlay use samples/spam --prefix")?;
    tester.run(code).expect_value_eq(expected)
}

#[test]
fn overlay_help_no_error() -> Result {
    let _: Value = test().run("overlay hide -h")?;
    let _: Value = test().run("overlay new -h")?;
    let _: Value = test().run("overlay use -h")?;
    Ok(())
}

#[test]
fn test_overlay_use_with_printing_file_pwd() -> Result {
    Playground::setup("use_with_printing_file_pwd", |dirs, nu| -> Result {
        let file = dirs.test().join("foo").join("mod.nu");
        nu.mkdir("foo").with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            "
                export-env {
                    $env.OVERLAY_FILE_PWD = $env.FILE_PWD
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("overlay use foo; $env.OVERLAY_FILE_PWD")
            .expect_value_eq(dirs.test().join("foo"))
    })
}

#[test]
fn test_overlay_use_with_printing_current_file() -> Result {
    Playground::setup("use_with_printing_current_file", |dirs, nu| -> Result {
        let file = dirs.test().join("foo").join("mod.nu");
        nu.mkdir("foo").with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            "
                export-env {
                    $env.OVERLAY_CURRENT_FILE = $env.CURRENT_FILE
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("overlay use foo; $env.OVERLAY_CURRENT_FILE")
            .expect_value_eq(dirs.test().join("foo").join("mod.nu"))
    })
}

#[test]
fn report_errors_in_export_env() -> Result {
    let commands = [
        r#"module spam { export-env { error make -u {msg: "reported"} } }"#,
        "overlay use spam",
    ];

    test().run(commands.join("; ")).expect_shell_error()?;
    let error = test().run_multiple(commands).expect_shell_error()?;
    assert_contains("reported", format!("{error:?}"));
    Ok(())
}

#[test]
fn overlay_hide_keeps_pwd() -> Result {
    let commands = [
        "overlay new spam",
        "cd samples",
        "overlay hide spam",
        "$env.PWD | path basename",
    ];

    test()
        .cwd("tests/overlays")
        .run(commands.join("; "))
        .expect_value_eq("samples")?;
    test()
        .cwd("tests/overlays")
        .run_multiple(commands)
        .expect_value_eq("samples")
}

#[test]
fn overlay_hide_keeps_pwd_set_by_export_env() -> Result {
    Playground::setup("overlay_hide_export_env_pwd", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("sub")
            .with_files(&[FileWithContent("spam.nu", "export-env { cd sub }")]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run("overlay hide spam")?;
        tester
            .run("$env.PWD | path basename")
            .expect_value_eq("sub")
    })
}

// The overlay's other variables still go away with it.
#[test]
fn overlay_hide_still_drops_other_env() -> Result {
    let commands = ["overlay new spam", "$env.FOO = 'foo'", "overlay hide spam"];

    let mut tester = test();
    for command in commands {
        let () = tester.run(command)?;
    }
    tester.run("'FOO' in $env").expect_value_eq(false)
}

// `overlay use <name>` reads the module's file again once it has changed.
#[rstest]
#[case::active("overlay_name_active", &[])]
#[case::hidden("overlay_name_hidden", &["overlay hide spam"])]
#[case::not_on_top("overlay_name_not_on_top", &["overlay new eggs"])]
fn overlay_use_by_name_reads_a_changed_file(
    #[case] topic: &str,
    #[case] between: &[&str],
) -> Result {
    Playground::setup(topic, |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export def foo [] { 'old' }; export-env { $env.SPAM = 'old' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        for command in between {
            let () = tester.run(command)?;
        }
        let () = tester.run(
            r#""export def foo [] { 'new' }; export-env { $env.SPAM = 'new' }" | save -f spam.nu"#,
        )?;
        let () = tester.run("overlay use spam")?;
        tester.run("foo").expect_value_eq("new")?;
        tester.run("$env.SPAM").expect_value_eq("new")
    })
}

#[test]
fn overlay_use_by_name_reads_a_changed_directory_module() -> Result {
    Playground::setup("overlay_name_dir", |dirs, sandbox| -> Result {
        sandbox.mkdir("pkg").with_files(&[
            FileWithContent("pkg/mod.nu", "export use sub.nu *"),
            FileWithContent("pkg/sub.nu", "export def foo [] { 'old' }"),
        ]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use pkg")?;
        tester.run("foo").expect_value_eq("old")?;
        let () = tester.run(r#""export def foo [] { 'new' }" | save -f pkg/sub.nu"#)?;
        let () = tester.run("overlay use pkg")?;
        tester.run("foo").expect_value_eq("new")
    })
}

// With an unchanged file, reactivating by name doesn't run `export-env` again.
#[test]
fn overlay_use_by_name_keeps_env_when_file_is_unchanged() -> Result {
    Playground::setup("overlay_name_unchanged", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export-env { $env.SPAM = 'module' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run("$env.SPAM = 'mine'")?;
        let () = tester.run("overlay use spam")?;
        tester.run("$env.SPAM").expect_value_eq("mine")
    })
}

// Reloading drops what the new version no longer exports, however the overlay comes back.
#[rstest]
#[case::path("overlay_deleted_path", &[], "overlay use spam.nu", "bar", "foo")]
#[case::name("overlay_deleted_name", &[], "overlay use spam", "bar", "foo")]
#[case::reload_flag("overlay_deleted_flag", &[], "overlay use --reload spam", "bar", "foo")]
#[case::hidden_path("overlay_deleted_hidden_path", &["overlay hide spam"], "overlay use spam.nu", "bar", "foo")]
#[case::hidden_name("overlay_deleted_hidden_name", &["overlay hide spam"], "overlay use spam", "bar", "foo")]
#[case::not_on_top("overlay_deleted_not_on_top", &["overlay new eggs"], "overlay use spam.nu", "bar", "foo")]
fn overlay_deleted_command_goes_away_on_reload(
    #[case] topic: &str,
    #[case] between: &[&str],
    #[case] again: &str,
    #[case] deleted: &str,
    #[case] kept: &str,
) -> Result {
    Playground::setup(topic, |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export def foo [] { 'old' }; export def bar [] { 'bar' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        for command in between {
            let () = tester.run(command)?;
        }
        let () = tester.run(r#""export def foo [] { 'new' }" | save -f spam.nu"#)?;
        let () = tester.run(again)?;
        tester.run(kept).expect_value_eq("new")?;
        tester
            .run(deleted)
            .expect_error_code_eq("nu::shell::external_command")
    })
}

#[test]
fn overlay_deleted_constant_goes_away_on_reload() -> Result {
    Playground::setup("overlay_deleted_const", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export const X = 'old'; export const Y = 'old'",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run("overlay hide spam")?;
        let () = tester.run_with_data("save -f spam.nu", "export const Y = 'new'")?;
        let () = tester.run("overlay use spam")?;
        tester.run("$Y").expect_value_eq("new")?;
        tester
            .run("$X")
            .expect_error_code_eq("nu::parser::variable_not_found")
    })
}

#[rstest]
#[case::reload("overlay_deleted_sub_reload", &[], "overlay use --reload spam")]
#[case::hidden_path("overlay_deleted_sub_path", &["overlay hide spam"], "overlay use spam.nu")]
#[case::hidden_name("overlay_deleted_sub_name", &["overlay hide spam"], "overlay use spam")]
fn overlay_deleted_submodule_goes_away_on_reload(
    #[case] topic: &str,
    #[case] before_the_edit: &[&str],
    #[case] again: &str,
) -> Result {
    Playground::setup(topic, |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export module sub { export def x [] { 'old' } }; export def foo [] { 'old' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run("use sub")?;
        for command in before_the_edit {
            let () = tester.run(*command)?;
        }
        let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
        let () = tester.run(again)?;
        tester.run("foo").expect_value_eq("new")?;
        tester
            .run("use sub")
            .expect_error_code_eq("nu::parser::module_not_found")
    })
}

#[test]
fn overlay_prefixed_deleted_command_goes_away_on_reload() -> Result {
    Playground::setup("overlay_deleted_prefixed", |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export def foo [] { 'old' }; export def bar [] { 'bar' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use --prefix spam.nu")?;
        let () = tester.run(r#""export def foo [] { 'new' }" | save -f spam.nu"#)?;
        let () = tester.run("overlay use --prefix spam.nu")?;
        tester.run("spam foo").expect_value_eq("new")?;
        tester
            .run("spam bar")
            .expect_error_code_eq("nu::shell::external_command")
    })
}

// What you defined inside the overlay isn't the module's, so a reload leaves it alone.
#[rstest]
#[case::custom_command("overlay_reload_custom", "def mine [] { 'mine' }", "mine", "mine")]
#[case::redefined("overlay_reload_redefined", "def bar [] { 'mine' }", "bar", "mine")]
fn overlay_reload_keeps_your_own_definitions(
    #[case] topic: &str,
    #[case] definition: &str,
    #[case] call: &str,
    #[case] expected: &str,
) -> Result {
    Playground::setup(topic, |dirs, sandbox| -> Result {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "export def foo [] { 'old' }; export def bar [] { 'bar' }",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("overlay use spam.nu")?;
        let () = tester.run(definition)?;
        let () = tester.run(r#""export def foo [] { 'new' }" | save -f spam.nu"#)?;
        let () = tester.run("overlay use spam.nu")?;
        tester.run(call).expect_value_eq(expected)
    })
}
