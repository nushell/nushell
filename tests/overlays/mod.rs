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
    let actual: Result<Value> = test().run(commands.iter().join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq(expected.clone())?;
    actual_repl.expect_value_eq(expected)?;
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
    let actual: Result<Value> = test().run(commands.iter().join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    actual_repl.expect_error_code_eq("nu::parser::overlay_prefix_mismatch")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bar")?;
    actual_repl.expect_value_eq("bar")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(format!("{actual_repl:?}").contains("Command `bar` not found"));
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

    let actual: Result<Value> = test().run(commands.join("; "));

    assert!(actual.is_err());
    Ok(())
}

#[test]
fn add_overlay_env() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use spam",
        "$env.FOO",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn add_prefixed_overlay_env_no_prefix() -> Result {
    let commands = [
        r#"module spam { export-env { $env.FOO = "foo" } }"#,
        "overlay use --prefix spam",
        "$env.FOO",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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
    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.iter().join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq(expected.clone())?;
    actual_repl.expect_value_eq(expected)?;
    Ok(())
}

#[test]
fn add_overlay_from_const_file_decl() -> Result {
    let commands = ["const file = 'samples/spam.nu'", "overlay use $file", "foo"];

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));

    actual.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("foo")?;
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

            let mut tester = test().cwd(dirs.test());
            tester
                .run(commands.join("; "))
                .expect_value_eq("[[a];[1]]")?;
            let mut repl_tester = test().cwd(dirs.test());
            commands
                .iter()
                .map(|line| repl_tester.run(*line))
                .try_fold(Value::test_nothing(), |_, value| value)
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

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn hide_overlay_from_const_name() -> Result {
    let commands = [
        "const mod = 'spam'",
        "overlay new $mod",
        "overlay hide $mod",
        "overlay list | where active == true | get name | str join ' '",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    let Value::String { val: actual, .. } = actual.unwrap() else {
        panic!("expected string value")
    };
    assert!(!actual.contains("spam"));
    Ok(())
}

// This one tests that the `nu_repl()` loop works correctly
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bar")?;
    actual_repl.expect_value_eq("bar")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bar")?;
    actual_repl.expect_value_eq("bar")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bar")?;
    actual_repl.expect_value_eq("bar")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn list_default_overlay() -> Result {
    let commands = ["overlay list | last | get name"];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("zero")?;
    actual_repl.expect_value_eq("zero")?;
    Ok(())
}

#[test]
fn list_last_overlay() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "overlay list | last | get name",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("spam")?;
    actual_repl.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn list_overlay_scoped() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam",
        "do { overlay list | last | get name }",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("spam")?;
    actual_repl.expect_value_eq("spam")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bagr")?;
    actual_repl.expect_value_eq("bagr")?;
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bagr")?;
    actual_repl.expect_value_eq("bagr")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bagr")?;
    actual_repl.expect_value_eq("bagr")?;
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bagr")?;
    actual_repl.expect_value_eq("bagr")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("new-foo")?;
    actual_repl.expect_value_eq("new-foo")?;
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_new() -> Result {
    let commands = ["overlay new spam", "overlay list | last | get name"];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("spam")?;
    actual_repl.expect_value_eq("spam")?;
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

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("samples")?;
    actual_repl.expect_value_eq("samples")?;
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

    let actual: Result<Value> = test()
        .cwd("tests/overlays/samples")
        .run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays/samples");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("samples")?;
    actual_repl.expect_value_eq("samples")?;
    Ok(())
}

#[test]
fn overlay_reactivate_with_module_name_should_change_pwd() -> Result {
    let commands = [
        "overlay use spam.nu",
        "cd ..",
        "overlay hide --keep-env [ PWD ] spam",
        "cd samples",
        "overlay use spam",
        "$env.PWD | path basename",
    ];

    let actual: Result<Value> = test()
        .cwd("tests/overlays/samples")
        .run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays/samples");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("overlays")?;
    actual_repl.expect_value_eq("overlays")?;
    Ok(())
}

#[test]
fn overlay_wrong_rename_type() -> Result {
    let commands = ["module spam {}", "overlay use spam as { echo foo }"];

    let actual: Result<Value> = test().run(commands.join("; "));

    assert!(actual.is_err());
    Ok(())
}

#[test]
fn overlay_add_renamed() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use spam as eggs --prefix",
        "eggs foo",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_add_renamed_from_file() -> Result {
    let commands = ["overlay use samples/spam.nu as eggs --prefix", "eggs foo"];

    let actual: Result<Value> = test().cwd("tests/overlays").run(commands.join("; "));
    let actual_repl = {
        let mut tester = test().cwd("tests/overlays");
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    actual_repl.expect_error_code_eq("nu::parser::cant_add_overlay_help")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foofoo")?;
    actual_repl.expect_value_eq("foofoo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    actual_repl.expect_error_code_eq("nu::shell::external_command")?;
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

    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual_repl.expect_value_eq(false)?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_use_export_env() -> Result {
    let commands = [
        "module spam { export-env { $env.FOO = 'foo' } }",
        "overlay use spam",
        "$env.FOO",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    let Value::String { val, .. } = actual.unwrap() else {
        panic!("expected string value")
    };
    assert_eq!(val.replace(",0 ", ".0 "), r#"["20.0 MB","20.0 MiB"]"#);
    let Value::String { val, .. } = actual_repl.unwrap() else {
        panic!("expected string value")
    };
    assert_eq!(val.replace(",0 ", ".0 "), r#"["20.0 MB","20.0 MiB"]"#);
    Ok(())
}

#[test]
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
        "$out | to json --raw",
    ];

    // Can't hide overlay within the same source file
    // let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    // actual.expect_value_eq(r#"["20.0 MB","20.0 MiB","20.0 MB"]"#)?;
    let Value::String { val, .. } = actual_repl.unwrap() else {
        panic!("expected string value")
    };
    assert_eq!(
        val.replace(",0 ", ".0 "),
        r#"["20.0 MB","20.0 MiB","20.0 MB"]"#
    );
    Ok(())
}

#[test]
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
        "$out | to json --raw",
    ];

    // Can't hide overlay within the same source file
    // let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    // actual.expect_value_eq(r#"["20.0 MB","20.0 MiB","20.0 MB"]"#)?;
    let Value::String { val, .. } = actual_repl.unwrap() else {
        panic!("expected string value")
    };
    assert_eq!(
        val.replace(",0 ", ".0 "),
        r#"["20.0 MB","20.0 MiB","20.0 MB","20.0 MiB"]"#
    );
    Ok(())
}

#[test]
fn overlay_use_export_env_hide() -> Result {
    let commands = [
        "$env.FOO = 'foo'",
        "module spam { export-env { hide-env FOO } }",
        "overlay use spam",
        "$env.FOO",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
    Ok(())
}

#[test]
fn overlay_use_do_cd() -> Result {
    Playground::setup("overlay_use_do_cd", |dirs, sandbox| -> Result {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env { cd test1/test2 }
                ",
            )]);

        let commands = [
            "overlay use test1/test2/spam.nu",
            "$env.PWD | path basename",
        ];

        let actual: Result<Value> = test().cwd(dirs.test()).run(commands.join("; "));

        actual.expect_value_eq("test2")?;
        Ok(())
    })
}

#[test]
fn overlay_use_do_cd_file_relative() -> Result {
    Playground::setup(
        "overlay_use_do_cd_file_relative",
        |dirs, sandbox| -> Result {
            sandbox
                .mkdir("test1/test2")
                .with_files(&[FileWithContentToBeTrimmed(
                    "test1/test2/spam.nu",
                    "
                    export-env { cd ($env.FILE_PWD | path join '..') }
                ",
                )]);

            let commands = [
                "overlay use test1/test2/spam.nu",
                "$env.PWD | path basename",
            ];

            let actual: Result<Value> = test().cwd(dirs.test()).run(commands.join("; "));

            actual.expect_value_eq("test1")?;
            Ok(())
        },
    )
}

#[test]
fn overlay_use_dont_cd_overlay() -> Result {
    Playground::setup("overlay_use_dont_cd_overlay", |dirs, sandbox| -> Result {
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

        let commands = ["source-env test1/test2/spam.nu", "$env.PWD | path basename"];

        let actual: Result<Value> = test().cwd(dirs.test()).run(commands.join("; "));

        actual.expect_value_eq("overlay_use_dont_cd_overlay")?;
        Ok(())
    })
}

#[test]
fn overlay_use_find_scoped_module() -> Result {
    Playground::setup("overlay_use_find_module_scoped", |dirs, _| -> Result {
        let commands = "
                do {
                    module spam { }

                    overlay use spam
                    overlay list | last | get name
                }
            ";

        let actual: Result<Value> = test().cwd(dirs.test()).run(commands);

        actual.expect_value_eq("spam")
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("bar")?;
    actual_repl.expect_value_eq("bar")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
    Ok(())
}

#[test]
fn overlay_trim_single_quote() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use 'spam'",
        "overlay list | last | get name",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_trim_single_quote_hide() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        "overlay use 'spam'",
        "overlay hide spam ",
        "foo",
    ];
    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
    Ok(())
}

#[test]
fn overlay_trim_double_quote() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        "overlay list | last | get name",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_trim_double_quote_hide() -> Result {
    let commands = [
        r#"module spam { export def foo [] { "foo" } }"#,
        r#"overlay use "spam" "#,
        "overlay hide spam ",
        "foo",
    ];
    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(actual_repl.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("new-bazbaz")?;
    actual_repl.expect_value_eq("new-bazbaz")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foofoofoo")?;
    actual_repl.expect_value_eq("foofoofoo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("newfoonewfoonewfoo")?;
    actual_repl.expect_value_eq("newfoonewfoonewfoo")?;
    Ok(())
}

#[test]
fn overlay_use_main() -> Result {
    let commands = [
        r#"module spam { export def main [] { "spam" } }"#,
        "overlay use spam",
        "spam",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_use_main_prefix() -> Result {
    let commands = [
        r#"module spam { export def main [] { "spam" } }"#,
        "overlay use spam --prefix",
        "spam",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_use_main_def_env() -> Result {
    let commands = [
        r#"module spam { export def --env main [] { $env.SPAM = "spam" } }"#,
        "overlay use spam",
        "spam",
        "$env.SPAM",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    actual.expect_value_eq("spam")?;
    Ok(())
}

#[test]
fn overlay_use_main_def_known_external() -> Result {
    // note: requires installed cargo
    let commands = [
        "module cargo { export extern main [] }",
        "overlay use cargo",
        "cargo --version",
    ];

    let actual: Result<Value> = test().inherit_rust_toolchain_env().run(commands.join("; "));

    let Value::String { val: actual, .. } = actual.unwrap() else {
        panic!("expected string value")
    };
    assert!(actual.contains("cargo"));
    Ok(())
}

#[test]
fn overlay_use_main_not_exported() -> Result {
    let commands = [
        r#"module my-super-cool-and-unique-module-name { def main [] { "hi" } }"#,
        "overlay use my-super-cool-and-unique-module-name",
        "my-super-cool-and-unique-module-name",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));

    assert!(actual.is_err());
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    actual_repl.expect_error_code_eq("nu::shell::external_command")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("foo")?;
    actual_repl.expect_value_eq("foo")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_ok());
    assert!(actual_repl.is_ok());
    actual.expect_value_eq("inner")?;
    actual_repl.expect_value_eq("inner")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_ok());
    assert!(actual_repl.is_ok());
    actual.expect_value_eq("inner")?;
    actual_repl.expect_value_eq("inner")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq("eggs")?;
    actual_repl.expect_value_eq("eggs")?;
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

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    actual.expect_value_eq(false)?;
    actual_repl.expect_value_eq(false)?;
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
    let actual: Result<Value> = test().cwd("tests/modules").run(commands.iter().join("; "));
    actual.expect_value_eq(expected)
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

        let actual: Result<Value> = test()
            .cwd(dirs.test())
            .run("overlay use foo; $env.OVERLAY_FILE_PWD");
        actual.expect_value_eq(dirs.test().join("foo").to_string_lossy())?;
        Ok(())
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

        let actual: Result<Value> = test()
            .cwd(dirs.test())
            .run("overlay use foo; $env.OVERLAY_CURRENT_FILE");
        actual.expect_value_eq(dirs.test().join("foo").join("mod.nu").to_string_lossy())?;
        Ok(())
    })
}

#[test]
fn report_errors_in_export_env() -> Result {
    let commands = [
        r#"module spam { export-env { error make -u {msg: "reported"} } }"#,
        "overlay use spam",
    ];

    let actual: Result<Value> = test().run(commands.join("; "));
    let actual_repl = {
        let mut tester = test();
        commands
            .iter()
            .map(|line| tester.run(*line))
            .try_fold(Value::test_nothing(), |_, value| value)
    };

    assert!(actual.is_err());
    assert!(format!("{actual_repl:?}").contains("reported"));
    Ok(())
}
