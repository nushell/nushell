use nu_test_support::prelude::*;

#[test]
fn env_change_define_command() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: 'def foo [] { "got foo!" }' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("foo").expect_value_eq("got foo!")
}

#[test]
fn env_change_define_variable() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: 'let x = "spam"' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$x").expect_value_eq("spam")
}

#[test]
fn env_change_define_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: '$env.SPAM = "spam"' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn env_change_define_alias() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: 'alias spam = echo "spam"' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("spam").expect_value_eq("spam")
}

#[test]
fn env_change_simple_block_preserve_env_var() -> Result {
    let mut tester = test();
    let () = tester
        .run_with_hooks(r#"$env.config.hooks.env_change.FOO = [{|| $env.SPAM = "spam" }]"#)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn env_change_simple_block_list_shadow_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            {|| $env.SPAM = "foo" }
            {|| $env.SPAM = "spam" }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn env_change_block_preserve_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: {|| $env.SPAM = "spam" } }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_prompt_define_command() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_prompt = [
            { code: 'def foo [] { "got foo!" }' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("foo").expect_value_eq("got foo!")
}

#[test]
fn pre_prompt_simple_block_preserve_env_var() -> Result {
    let mut tester = test();
    let hook_config = r#"$env.config.hooks.pre_prompt = [{|| $env.SPAM = "spam" }]"#;
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_prompt_simple_block_list_shadow_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_prompt = [
            {|| $env.SPAM = "foo" }
            {|| $env.SPAM = "spam" }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_prompt_block_preserve_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_prompt = [
            { code: {|| $env.SPAM = "spam" } }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_execution_define_command() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_execution = [
            { code: 'def foo [] { "got foo!" }' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("foo").expect_value_eq("got foo!")
}

#[test]
fn pre_execution_simple_block_preserve_env_var() -> Result {
    let mut tester = test();
    let hook_config = r#"$env.config.hooks.pre_execution = [{|| $env.SPAM = "spam" }]"#;
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_execution_simple_block_list_shadow_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_execution = [
            {|| $env.SPAM = "foo" }
            {|| $env.SPAM = "spam" }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_execution_block_preserve_env_var() -> Result {
    let hook_config = r#"
        $env.config.hooks.pre_execution = [
            { code: {|| $env.SPAM = "spam" } }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn pre_execution_commandline() -> Result {
    let mut tester = test();
    let hook_config = "
        $env.config.hooks.pre_execution = [{ code: {|| $env.repl_commandline = (commandline) } }]
    ";
    let () = tester.run_with_hooks(hook_config)?;

    tester
        .run_with_hooks("$env.repl_commandline")
        .expect_value_eq("$env.repl_commandline")
}

#[test]
fn env_change_shadow_command() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: 'def foo [] { "got spam!" }' }
            { code: 'def foo [] { "got foo!" }' }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("foo").expect_value_eq("got foo!")
}

#[test]
fn env_change_block_dont_preserve_command() -> Result {
    let hook_config = r#"
        $env.config.hooks.env_change.FOO = [
            { code: {|| def foo [] { "foo" } } }
        ]
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester
        .run_with_hooks::<Value>("foo")
        .expect_error_code_eq("nu::shell::external_command")
}

#[test]
fn env_change_block_condition_pwd() -> Result {
    let hook_config = "
        $env.config.hooks.env_change.PWD = [
            {
                condition: {|before, after| ($after | path basename) == samples }
                code: 'source-env .nu-env'
            }
        ]
    ";
    let mut tester = test().cwd("tests/hooks");
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("cd samples")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn env_change_block_condition_pwd_is_case_insensitive() -> Result {
    let hook_config = "
        $env.config.hooks.env_change.pWD = [
            {
                condition: {|before, after| ($after | path basename) == samples }
                code: 'source-env .nu-env'
            }
        ]
    ";
    let mut tester = test().cwd("tests/hooks");
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("cd samples")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn env_change_block_condition_correct_args() -> Result {
    let hook_config = "
        $env.config.hooks.env_change.FOO = [
            {
                condition: {|before, after| $before == 1 and $after == 2}
                code: {|before, after| $env.SPAM = ($before == 1 and $after == 2) }
            }
        ]
    ";
    let mut tester = test();
    let () = tester.run_with_hooks("$env.FOO = 1")?;
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("")?;
    let () = tester.run_with_hooks("$env.FOO = 2")?;

    tester.run_with_hooks("$env.SPAM").expect_value_eq(true)
}

#[test]
fn env_change_dont_panic_with_many_args() -> Result {
    let mut tester = test();
    let hook_config =
        "$env.config.hooks.env_change.FOO = [{ code: { |a, b, c| $env.SPAM = 'spam' } }]";
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester
        .run_with_hooks::<Value>("")
        .expect_error_code_eq("nu::shell::incompatible_parameters")
}

#[test]
fn err_hook_wrong_env_type_1() -> Result {
    let code = "
        $env.config = {
            hooks: {
                env_change: {
                    FOO : 1
                }
            }
        }
    ";

    test()
        .run_with_hooks::<Value>(code)
        .expect_error_code_eq("nu::shell::invalid_config")
}

#[test]
fn err_hook_wrong_env_type_2() -> Result {
    let code = r#"
        $env.config = {
            hooks: {
                env_change: "print spam"
            }
        }
    "#;

    test()
        .run_with_hooks::<Value>(code)
        .expect_error_code_eq("nu::shell::invalid_config")
}

#[test]
fn err_hook_wrong_env_type_3() -> Result {
    let code = "
        $env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        code: 1
                    }
                }
            }
        }
    ";

    test()
        .run_with_hooks::<Value>(code)
        .expect_error_code_eq("nu::shell::invalid_config")
}

#[test]
fn err_hook_non_boolean_condition_output() -> Result {
    let code = r#"
        $env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        condition: {|| "foo" }
                        code: "print spam"
                    }
                }
            }
        }
    "#;

    test()
        .run_with_hooks::<Value>(code)
        .expect_error_code_eq("nu::shell::invalid_config")
}

#[test]
fn err_hook_non_condition_not_a_block() -> Result {
    let code = r#"
        $env.config = {
            hooks: {
                env_change: {
                    FOO : {
                        condition: "foo"
                        code: "print spam"
                    }
                }
            }
        }
    "#;

    test()
        .run_with_hooks::<Value>(code)
        .expect_error_code_eq("nu::shell::invalid_config")
}

#[test]
fn err_hook_parse_error() -> Result {
    let code = r#"
        $env.config = {
            hooks: {
                env_change: {
                    FOO: [{
                        code: "def foo { 'foo' }"
                    }]
                }
            }
        }
    "#;
    let mut tester = test();
    let () = tester.run_with_hooks(code)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    let err = tester.run_with_hooks::<Value>("").expect_shell_error()?;
    assert_contains("source code has errors", err.generic_msg()?);
    Ok(())
}

#[test]
fn env_change_overlay() -> Result {
    let hook_config = "$env.config.hooks.env_change.FOO = [{ code: 'overlay use test' }]";
    let mut tester = test();
    let () = tester.run_with_hooks("module test { export-env { $env.BAR = 2 } }")?;
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;

    tester.run_with_hooks("$env.BAR").expect_value_eq(2)
}
