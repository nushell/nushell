use nu_experimental::PIPE_FAIL;
use nu_test_support::prelude::*;

#[test]
fn try_succeed() -> Result {
    test()
        .run("try { 345 } catch { 'hello' }")
        .expect_value_eq(345)
}

#[test]
fn try_catch() -> Result {
    test()
        .run("try { foobarbaz } catch { 'hello' }")
        .expect_value_eq("hello")
}

#[test]
fn catch_can_access_error() -> Result {
    test()
        .run("try { foobarbaz } catch { |err| $err | get raw }")
        .expect_error_code_eq("nu::shell::external_command")
}

#[test]
fn catch_can_access_error_as_dollar_in() -> Result {
    test()
        .run("try { foobarbaz } catch { $in | get raw }")
        .expect_error_code_eq("nu::shell::external_command")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn external_failed_should_be_caught() -> Result {
    test()
        .run("try { fail; 'success' } catch { 'fail' }")
        .expect_value_eq("fail")
}

#[test]
fn loop_try_break_should_be_successful() -> Result {
    test()
        .run("loop { try { break } catch { 'failed'; continue } }; 'successful'")
        .expect_value_eq("successful")
}

#[test]
fn loop_try_break_should_pop_error_handlers() -> Result {
    let code = r#"
        do {
            loop {
                try {
                    break
                } catch {
                    return 'jumped to catch block'
                }
            }
            error make -u {msg: "success"}
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("success", err.to_string());
    Ok(())
}

#[test]
fn loop_nested_try_break_should_pop_error_handlers() -> Result {
    let code = r#"
        do {
            loop {
                try {
                    try {
                        break
                    } catch {
                        return 'jumped to inner catch block'
                    }
                } catch {
                    return 'jumped to outer catch block'
                }
            }
            error make -u {msg: "success"}
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("success", err.to_string());
    Ok(())
}

#[test]
fn loop_try_continue_should_pop_error_handlers() -> Result {
    let code = r#"
        do {
            mut error = false

            loop {
                if $error {
                    error make -u {msg: "success"}
                }

                try {
                    $error = true
                    continue
                } catch {
                    return 'jumped to catch block'
                }
            }
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("success", err.to_string());
    Ok(())
}

#[test]
fn loop_catch_break_should_show_failed() -> Result {
    let code = "
        loop {
            try { invalid 1; continue } catch { break }
        }
        'failed'
    ";

    test().run(code).expect_value_eq("failed")
}

#[test]
fn loop_try_ignores_continue() -> Result {
    let code = "
        mut total = 0
        for i in 0..10 {
            try {
                if ($i mod 2) == 0 { continue }
                $total += 1
            } catch {
                break
            }
        }
        $total
    ";

    test().run(code).expect_value_eq(5)
}

#[test]
fn loop_try_break_on_command_should_show_successful() -> Result {
    test()
        .run("loop { try { ls; break } catch { 'failed'; continue } }")
        .expect_value_eq(())
}

#[test]
fn catch_block_can_use_error_object() -> Result {
    test()
        .run("try {1 / 0} catch {|err| $err | get msg}")
        .expect_value_eq("Division by zero.")
}

#[test]
fn catch_input_type_mismatch_and_rethrow() -> Result {
    let err = test()
        .run("let x: any = 1; try { $x | get 1 } catch {|err| error make { msg: ($err | get msg) } }")
        .expect_error()?;
    assert_contains("Input type not supported", err.to_string());
    Ok(())
}

// This test is disabled on Windows because they cause a stack overflow in CI (but not locally!).
// For reasons we don't understand, the Windows CI runners are prone to stack overflow.
// TODO: investigate so we can enable on Windows
#[cfg(not(target_os = "windows"))]
#[test]
fn can_catch_infinite_recursion() -> Result {
    test()
        .run(r#"def bang [] { try { bang } catch { "Caught infinite recursion" } }; bang"#)
        .expect_value_eq("Caught infinite recursion")
}

#[test]
#[deps(NU)]
fn exit_code_available_in_catch_env() -> Result {
    test()
        .run("try { nu -c 'exit 42' } catch { $env.LAST_EXIT_CODE }")
        .expect_value_eq(42)
}

#[test]
#[deps(NU)]
fn exit_code_available_in_catch() -> Result {
    test()
        .run("try { nu -c 'exit 42' } catch { |e| $e.exit_code }")
        .expect_value_eq(42)
}

#[test]
#[deps(NU)]
fn catches_exit_code_in_assignment() -> Result {
    test()
        .run("let x = try { nu -c 'exit 42' } catch { |e| $e.exit_code }; $x")
        .expect_value_eq(42)
}

#[test]
#[deps(NU)]
fn catches_exit_code_in_expr() -> Result {
    test()
        .run("try { nu -c 'exit 42' } catch { |e| $e.exit_code }")
        .expect_value_eq(42)
}

#[test]
fn prints_only_if_last_pipeline() -> Result {
    test()
        .run("try { 'should not print' }; 'last value'")
        .expect_value_eq("last value")?;

    test()
        .run("try { ['should not print'] | every 1 }; 'last value'")
        .expect_value_eq("last value")
}

#[test]
fn get_error_columns() -> Result {
    test()
        .run(" try { non_existent_command } catch { columns }")
        .expect_value_eq(["msg", "debug", "raw", "rendered", "details"])
}

#[test]
fn get_json_error() -> Result {
    let empty_list = [(); 0];
    test()
        .run("try { non_existent_command } catch { get details | reject labels.span }")
        .expect_value_eq(test_record! {
            "msg" => "External command failed",
            "labels" => [
                test_record! {
                    "text" => "Command `non_existent_command` not found",
                    "location" => test_record! {
                        "file" => "nu-tester-0",
                        "start" => 6,
                        "end" => 26,
                    },
                }
            ],
            "code" => "nu::shell::external_command",
            "url" => (),
            "help" => "`non_existent_command` is neither a Nushell built-in or a known external command",
            "inner" => empty_list,
        })
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn pipefail_works() -> Result {
    test()
        .run("fail | lines | length; 'bbb'")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn let_ignores_pipefail() -> Result {
    test()
        .run("try { let x = fail | lines | length; $x } catch {|e| $e.exit_code}")
        .expect_value_eq(0)
}

#[test]
fn try_catch_finally() -> Result {
    test()
        .run("try { 1 / 0 } catch { 'inside catch' } finally { 'this finally' }")
        .expect_value_eq("inside catch")?;

    test()
        .run("try { 'inside try' } catch { 'inside catch' } finally { 'this finally' }")
        .expect_value_eq("inside try")?;

    let err = test()
        .run("try { 1 / 0 } catch { 1 / 0; 'inside catch' } finally { 'this finally' }")
        .expect_error()?;
    assert_contains("division by zero", err.to_string().to_lowercase());
    Ok(())
}

#[test]
fn try_finally() -> Result {
    test().run("try { 0 } finally { 3 }").expect_value_eq(0)?;

    let err = test()
        .run("try { 1 / 0 } finally { 'this finally' }")
        .expect_error()?;
    assert_contains("division by zero", err.to_string().to_lowercase());

    test()
        .run("try { 'inside try' } finally { 'this finally' }")
        .expect_value_eq("inside try")
}

#[test]
fn finally_should_run_before_return() -> Result {
    test()
        .run("def aa [] { try { return 3 } finally { 'this finally' } }; let x = aa; $x == 3")
        .expect_value_eq(true)?;

    test()
        .run("def aa [] { try { 1 / 0 } catch { return 44 } finally { 'this finally' } }; let x = aa; $x == 44")
        .expect_value_eq(true)
}

#[test]
fn return_statement_in_finally_should_be_used() -> Result {
    test()
        .run("def aa [] { try { return 3 } finally { return 4 } }; let x = aa; $x == 4")
        .expect_value_eq(true)
}

#[test]
fn try_finally_with_variable() -> Result {
    test()
        .run("try { 1 / 0 } finally {|x| $x.msg }")
        .expect_error_code_eq("nu::shell::division_by_zero")?;

    test()
        .run("let x = try { 3 } finally {|x| $x == 3 }; $x")
        .expect_value_eq(3)
}

#[test]
#[deps(NU)]
fn try_exit_runs_finally() -> Result {
    let code = "try { exit 3 } finally { print 'this finally' }";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_eq!(result.stdout.trim_end(), "this finally");
    assert_eq!(result.exit_code, 3);

    let code = "
        try {
            try {
                exit 3
            } finally {
                print 'inner finally'
            }
        } finally {
            print 'outer finally'
        }
    ";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_contains("inner finally", &result.stdout);
    assert_contains("outer finally", &result.stdout);
    assert_eq!(result.exit_code, 3);
    Ok(())
}

#[test]
#[deps(NU)]
fn try_abort_not_run_finally() -> Result {
    let code = "try { exit 3 --abort} finally { print 'this finally' }";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_contains_not("this finally", &result.stdout);
    assert_eq!(result.exit_code, 3);
    Ok(())
}

#[test]
fn catch_finally_with_variable() -> Result {
    test()
        .run("try { 1 / 0 } catch { 33 } finally {|x| $x == 33 }")
        .expect_value_eq(33)?;

    test()
        .run("try { 1 / 0 } catch { 33; error make 'err in catch' } finally {|x| $x.msg == 'err in catch'}")
        .expect_error_code_eq("nu::shell::error")
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_ECHO_ENV)]
fn finally_should_not_run_before_try_finished() -> Result {
    let code = "
        with-env { FOO: 'bar' } {
            try { echo_env FOO } finally { 'bb' }
        }
    ";

    test().run(code).expect_value_eq("bar")
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_ECHO_ENV)]
fn finally_should_not_run_before_catch_finished() -> Result {
    let code = "
        with-env { FOO: 'bar' } {
            try { 1 / 0 } catch { echo_env FOO } finally { 'bb' }
        }
    ";

    test().run(code).expect_value_eq("bar")
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn finally_should_not_run_twice_when_error_in_finally() -> Result {
    let code = r#"
        try {
            fail 0
        } finally {
            error make -u "oh no"
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("oh no", err.to_string());
    Ok(())
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn try_wont_generate_extra_output() -> Result {
    test()
        .run("try { fail | is-empty } catch { 'here' }")
        .expect_value_eq("here")
}

#[test]
#[exp(PIPE_FAIL)]
fn try_wont_run_twice_when_no_catch_and_finally_block() -> Result {
    let code = r#"
        do {
            try {}
            print "aa"
            not_real_cmd
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("External command failed", err.to_string());
    Ok(())
}

#[test]
#[exp(PIPE_FAIL)]
fn try_with_just_finally_wont_pop_enclosing_error_handler() -> Result {
    let code = r#"
        try {
            try { print "inner" } finally { print "finally" }
            error make { msg: "error" }
        }
        "outer"
    "#;

    test().run(code).expect_value_eq("outer")
}
