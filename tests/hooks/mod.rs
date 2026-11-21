use nu_cmd_base::hook::eval_hook;
use nu_protocol::{PipelineData, Span};
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

// A failing env_change hook runs once per change, as one that succeeds does, rather than again
// before every prompt.
#[test]
fn failing_env_change_hook_runs_once_per_change() -> Result {
    let mut tester = test();
    let () = tester
        .run_with_hooks("$env.config.hooks.env_change.FOO = [{|| error make {msg: boom} }]")?;
    let () = tester.run_with_hooks("$env.FOO = 1")?;
    tester
        .run_with_hooks::<Value>("null")
        .expect_shell_error()?;

    tester.run_with_hooks("null").expect_value_eq(())
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

#[rstest::rstest]
#[case::closure_appended_to_null("null | append {|cmd| $'($cmd) from hook' }")]
#[case::printing_hook_first("[{|cmd| print hi } {|cmd| $'($cmd) from hook' }]")]
fn command_not_found_hook_list_output_becomes_help(#[case] hook: &str) -> Result {
    let code = format!("$env.config.hooks.command_not_found = ({hook}); ferris_is_not_here");
    let err = test().run(code).expect_shell_error()?;
    pretty_assertions::assert_matches!(
        err,
        ShellError::ExternalCommand { help, .. } if help == "ferris_is_not_here from hook"
    );
    Ok(())
}

// A failure inside a hook reaches the REPL in `$env.LAST_EXIT_CODE`, also when the hook catches it.
// A hook with no failures leaves the code alone, and an assignment is kept. The tester doesn't set
// the code itself, so each case starts from 3.
#[rstest::rstest]
#[case::caught_closure(
    "$env.config.hooks.pre_prompt = [{|| try { error make {msg: boom} } catch { null } }]",
    1
)]
#[case::caught_string(
    r#"$env.config.hooks.pre_prompt = ["try { error make {msg: boom} } catch { null }"]"#,
    1
)]
#[case::caught_record_string(
    r#"$env.config.hooks.pre_prompt = [{ code: "try { error make {msg: boom} } catch { null }" }]"#,
    1
)]
#[case::try_without_catch(
    "$env.config.hooks.pre_prompt = [{|| try { error make {msg: boom} } }]",
    1
)]
#[case::pre_execution(
    "$env.config.hooks.pre_execution = [{|| try { error make {msg: boom} } catch { null } }]",
    1
)]
#[case::env_change(
    "$env.config.hooks.env_change.FOO = [{|| try { error make {msg: boom} } catch { null } }]",
    1
)]
#[case::assignment("$env.config.hooks.pre_prompt = [{|| $env.LAST_EXIT_CODE = 0 }]", 0)]
#[case::no_failure("$env.config.hooks.pre_prompt = [{|| $env.SPAM = 'spam' }]", 3)]
fn hook_exit_code_reaches_repl(#[case] hook_config: &str, #[case] expected: i64) -> Result {
    let mut tester = test();
    let () = tester.run_with_hooks(hook_config)?;
    let () = tester.run("$env.FOO = 1; $env.LAST_EXIT_CODE = 3")?;

    tester
        .run_with_hooks("$env.LAST_EXIT_CODE")
        .expect_value_eq(expected)
}

// A hook that fails without catching the failure still sets the code, although a closure hook's
// own environment changes are dropped when it fails. An external that ends a hook, maybe as the
// only thing in it, fails the hook too.
#[rstest::rstest]
#[case::closure("{|| fail 42; null }")]
#[case::record_closure("{code: {|| fail 42; null }}")]
#[case::closure_ending_in_external("{|| fail 42 }")]
#[case::record_closure_ending_in_external("{code: {|| fail 42 }}")]
#[case::string_ending_in_external("'fail 42'")]
#[case::string("'fail 42; null'")]
#[case::record_string("{code: 'fail 42; null'}")]
#[nu_test_support::test]
#[deps(TESTBIN_FAIL)]
fn uncaught_hook_failure_sets_last_exit_code(#[case] hook: &str) -> Result {
    let mut tester = test();
    let () = tester.run_with_hooks(format!("$env.config.hooks.pre_prompt = [{hook}]"))?;
    let () = tester.run("$env.LAST_EXIT_CODE = 3")?;
    tester.run_with_hooks("null").expect_shell_error()?;

    tester.run("$env.LAST_EXIT_CODE").expect_value_eq(42)
}

// Every hook form gets the input it is called with, which is how `display_output` sees the value
// to print.
#[rstest::rstest]
#[case::string("'$in | describe'")]
#[case::closure("{|| $in | describe }")]
#[case::record_string("{code: '$in | describe'}")]
#[case::record_closure("{code: {|| $in | describe }}")]
fn hook_gets_its_input(#[case] hook: &str) -> Result {
    let mut tester = test();
    let hook: Value = tester.run(hook)?;
    let input = PipelineData::value(test_value!([1, 2, 3]), None);

    let output = eval_hook(
        &mut tester.engine_state,
        &mut tester.stack,
        Some(input),
        &[],
        &hook,
        "display_output",
    )?;
    assert_eq!(
        output.into_value(Span::test_data())?,
        Value::test_string("list<int>")
    );
    Ok(())
}

// Every hook form clears the caller's pipe redirection, so an external in a `command_not_found`
// hook, which runs while the caller collects a value, writes to the terminal rather than into the
// hook's output.
#[rstest::rstest]
#[case::string("'cococo from-hook'")]
#[case::closure("{|| cococo from-hook }")]
#[case::record_string("{code: 'cococo from-hook'}")]
#[case::record_closure("{code: {|| cococo from-hook }}")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn hook_does_not_collect_external_output(#[case] hook: &str) -> Result {
    let mut tester = test();
    let hook: Value = tester.run(hook)?;

    let output = eval_hook(
        &mut tester.engine_state,
        &mut tester.stack.start_collect_value(),
        None,
        &[],
        &hook,
        "command_not_found",
    )?;
    // The external's stdout went to the terminal, which leaves an empty string as its value.
    assert_eq!(
        output.into_value(Span::test_data())?,
        Value::test_string("")
    );
    Ok(())
}
