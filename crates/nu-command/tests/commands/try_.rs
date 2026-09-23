use nu_experimental::PIPE_FAIL;
use nu_test_support::prelude::*;
use rstest::rstest;

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

#[test]
#[deps(NU)]
fn can_catch_infinite_recursion() -> Result {
    let runner = "let commands = $in; nu -n -c $commands";
    let code = r#"def bang [] { try { bang } catch { "Caught infinite recursion" } }; bang"#;
    test()
        .run_with_data(runner, code)
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
            let _ = "aa"
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
            try { let _ = "inner" } finally { let _ = "finally" }
            error make { msg: "error" }
        }
        "outer"
    "#;

    test().run(code).expect_value_eq("outer")
}

// The cases below record the order in which `try`, `catch` and `finally` blocks run in
// `$env.LOG`, so that the handler order (and not just the final value) is asserted. `finally`
// blocks are closures, so a `mut` variable can't be updated there, but the environment can.
#[rstest]
#[case::nested_finally_runs_before_outer_catch(
    r#"
        try {
            try {
                $env.LOG ++= ["try"]
                error make { msg: "bad" }
            } finally {
                $env.LOG ++= ["inner finally"]
            }
        } catch {
            $env.LOG ++= ["outer catch"]
        }
    "#,
    vec!["try", "inner finally", "outer catch"]
)]
#[case::error_after_finally_does_not_continue_the_block(
    r#"
        try {
            try { error make { msg: "bad" } } finally { $env.LOG ++= ["finally"] }
            $env.LOG ++= ["after"]
        } catch {
            $env.LOG ++= ["caught"]
        }
    "#,
    vec!["finally", "caught"]
)]
#[case::error_in_catch_runs_finally_before_outer_catch(
    r#"
        try {
            try {
                error make { msg: "a" }
            } catch {
                error make { msg: "b" }
            } finally {
                $env.LOG ++= ["finally"]
            }
        } catch {|err|
            $env.LOG ++= [$"caught ($err.msg)"]
        }
    "#,
    vec!["finally", "caught b"]
)]
#[case::rethrow_in_catch_runs_finally_before_outer_catch(
    r#"
        try {
            try {
                error make { msg: "a" }
            } catch {|err|
                error make { msg: $"re-($err.msg)" }
            } finally {
                $env.LOG ++= ["finally"]
            }
        } catch {|err|
            $env.LOG ++= [$err.msg]
        }
    "#,
    vec!["finally", "re-a"]
)]
#[case::error_in_finally_replaces_the_original_error(
    r#"
        try {
            try { error make { msg: "a" } } finally { error make { msg: "b" } }
        } catch {|err|
            $env.LOG ++= [$"caught ($err.msg)"]
        }
    "#,
    vec!["caught b"]
)]
#[case::finally_parameter_sees_the_error_when_nested(
    r#"
        try {
            try { error make { msg: "a" } } finally {|err| $env.LOG ++= [$"finally ($err.msg)"] }
        } catch {|err|
            $env.LOG ++= [$"caught ($err.msg)"]
        }
    "#,
    vec!["finally a", "caught a"]
)]
#[case::finally_parameter_is_nothing_on_return(
    "
        def --env foo [] {
            try { return 1 } finally {|value| $env.LOG ++= [($value | describe)] }
        }
        foo
    ",
    vec!["nothing"]
)]
#[case::every_nested_finally_runs_in_order(
    r#"
        try {
            try {
                try { error make { msg: "a" } } finally { $env.LOG ++= ["1"] }
            } finally {
                $env.LOG ++= ["2"]
            }
        } catch {
            $env.LOG ++= ["3"]
        }
    "#,
    vec!["1", "2", "3"]
)]
#[case::error_in_finally_with_nested_catch_does_not_continue_the_block(
    r#"
        try {
            try {
                error make { msg: "a" }
            } finally {
                try { error make { msg: "b" } } catch { $env.LOG ++= ["inner caught"] }
            }
            $env.LOG ++= ["after"]
        } catch {|err|
            $env.LOG ++= [$"caught ($err.msg)"]
        }
    "#,
    vec!["inner caught", "caught a"]
)]
#[case::finally_inside_finally_on_error(
    r#"
        try {
            try {
                error make { msg: "a" }
            } finally {
                try { $env.LOG ++= ["inner try"] } finally { $env.LOG ++= ["inner finally"] }
                $env.LOG ++= ["outer finally"]
            }
            $env.LOG ++= ["after"]
        } catch {
            $env.LOG ++= ["caught"]
        }
    "#,
    vec!["inner try", "inner finally", "outer finally", "caught"]
)]
#[case::return_through_nested_finally_skips_the_rest_of_the_block(
    r#"
        def --env foo [] {
            try {
                try { return 1 } finally { $env.LOG ++= ["inner"] }
                $env.LOG ++= ["after"]
            } finally {
                $env.LOG ++= ["outer"]
            }
            2
        }
        let value = foo
        $env.LOG ++= [$value]
    "#,
    vec![Value::test_string("inner"), Value::test_string("outer"), Value::test_int(1)]
)]
#[case::return_in_try_skips_catch_but_runs_finally(
    r#"
        def --env foo [] {
            try { return 7 } catch { $env.LOG ++= ["catch"] } finally { $env.LOG ++= ["finally"] }
        }
        let value = foo
        $env.LOG ++= [$value]
    "#,
    vec![Value::test_string("finally"), Value::test_int(7)]
)]
#[case::return_in_nested_finally_wins_over_the_error(
    r#"
        def --env foo [] {
            try {
                try { error make { msg: "a" } } finally { return 5 }
            } finally {
                $env.LOG ++= ["outer"]
            }
        }
        let value = foo
        $env.LOG ++= [$value]
    "#,
    vec![Value::test_string("outer"), Value::test_int(5)]
)]
#[case::return_value_stream_is_collected_before_nested_finally(
    r#"
        def --env foo [] {
            try {
                try { return (1..3 | each { $in * 10 }) } finally { $env.LOG ++= ["inner"] }
            } finally {
                $env.LOG ++= ["outer"]
            }
        }
        let value = foo
        $env.LOG ++= [$value]
    "#,
    vec![
        Value::test_string("inner"),
        Value::test_string("outer"),
        Value::test_list(vec![Value::test_int(10), Value::test_int(20), Value::test_int(30)]),
    ]
)]
#[case::handlers_of_a_called_command_run_before_the_callers(
    r#"
        def --env boom [] {
            try { error make { msg: "a" } } finally { $env.LOG ++= ["def finally"] }
        }
        try {
            boom
        } catch {|err|
            $env.LOG ++= [$"caught ($err.msg)"]
        } finally {
            $env.LOG ++= ["caller finally"]
        }
    "#,
    vec!["def finally", "caught a", "caller finally"]
)]
#[case::break_in_try_runs_finally(
    r#"
        for x in [1 2] {
            try { $env.LOG ++= [$"try ($x)"]; break } finally { $env.LOG ++= ["finally"] }
        }
    "#,
    vec!["try 1", "finally"]
)]
#[case::continue_in_try_runs_finally(
    r#"
        for x in [1 2] {
            try { continue } finally { $env.LOG ++= [$"finally ($x)"] }
            $env.LOG ++= ["after"]
        }
    "#,
    vec!["finally 1", "finally 2"]
)]
#[case::break_in_try_skips_catch_but_runs_finally(
    r#"
        for x in [1 2] {
            try { break } catch { $env.LOG ++= ["catch"] } finally { $env.LOG ++= ["finally"] }
        }
    "#,
    vec!["finally"]
)]
#[case::break_in_catch_runs_finally(
    r#"
        for x in [1 2] {
            try { error make { msg: "a" } } catch { break } finally { $env.LOG ++= ["finally"] }
        }
    "#,
    vec!["finally"]
)]
#[case::break_in_finally_abandons_the_error(
    r#"
        for x in [1 2] {
            try { error make { msg: "a" } } finally { $env.LOG ++= [$"finally ($x)"]; break }
        }
        $env.LOG ++= ["done"]
    "#,
    vec!["finally 1", "done"]
)]
#[case::break_through_nested_try_finally(
    r#"
        for x in [1 2] {
            try {
                try { break } finally { $env.LOG ++= ["inner"] }
            } finally {
                $env.LOG ++= ["outer"]
            }
        }
    "#,
    vec!["inner", "outer"]
)]
#[case::break_in_try_finally_leaves_no_stale_finally_behind(
    r#"
        for x in [1 2] {
            try { break } finally { $env.LOG ++= ["finally"] }
        }
        try { error make { msg: "boom" } } catch { $env.LOG ++= ["caught"] }
    "#,
    vec!["finally", "caught"]
)]
#[case::break_in_try_finally_keeps_the_enclosing_catch(
    r#"
        try {
            for x in [1] {
                try { break } finally { $env.LOG ++= ["finally"] }
            }
            error make { msg: "boom" }
        } catch {
            $env.LOG ++= ["caught"]
        }
    "#,
    vec!["finally", "caught"]
)]
#[case::break_unwinds_every_handler_between_the_loop_and_the_break(
    r#"
        for x in [1] {
            try {
                try {
                    try { break } catch {} finally { $env.LOG ++= ["a"] }
                } finally {
                    $env.LOG ++= ["b"]
                }
            } catch {} finally {
                $env.LOG ++= ["c"]
            }
        }
        try { error make { msg: "boom" } } catch { $env.LOG ++= ["caught"] }
    "#,
    vec!["a", "b", "c", "caught"]
)]
fn try_handlers_run_in_order(#[case] code: &str, #[case] expected: Vec<impl IntoValue>) -> Result {
    test()
        .run_multiple(["$env.LOG = []", code, "$env.LOG"])
        .expect_value_eq(expected)
}

#[test]
fn error_in_try_with_only_finally_still_propagates_after_nested_finally() -> Result {
    let code = r#"
        try {
            try { error make { msg: "bad" } } finally { "inner" }
        } finally {
            "outer"
        }
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("bad", err.to_string());
    Ok(())
}

#[test]
fn nested_try_finally_value_is_seen_by_outer_catch() -> Result {
    test()
        .run("try { try { error make { msg: 'a' } } finally { 'ignored' } } catch { 'caught' }")
        .expect_value_eq("caught")?;

    test()
        .run("try { try { 5 } finally { 'ignored' } } finally { 'ignored too' }")
        .expect_value_eq(5)
}

#[test]
#[deps(NU)]
fn exit_in_nested_try_skips_catch_but_runs_finally() -> Result {
    let code = "
        try {
            try {
                exit 5
            } finally {
                print 'inner finally'
            }
        } catch {
            print 'catch'
        }
        print 'after'
    ";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_eq!(result.stdout.trim_end(), "inner finally");
    assert_eq!(result.exit_code, 5);

    let code = "
        try { error make { msg: 'a' } } catch { exit 2 } finally { print 'finally' }
        print 'after'
    ";
    let result: CompleteResult =
        test().run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_eq!(result.stdout.trim_end(), "finally");
    assert_eq!(result.exit_code, 2);
    Ok(())
}
