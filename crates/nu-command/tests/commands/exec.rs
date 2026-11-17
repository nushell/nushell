use nu_test_support::prelude::*;

#[test]
#[deps(TESTBIN_COCOCO)]
fn basic_exec() -> Result {
    test()
        .run("nu -n -c 'exec cococo a b c'")
        .expect_value_eq("a b c")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn exec_complex_args() -> Result {
    test()
        .run("nu -n -c 'exec cococo b -- --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --'")
        .expect_value_eq("b --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn exec_fail_batched_short_args() -> Result {
    let code = "
        nu -n -c 'exec cococo -ab 10'
        | complete
    ";
    let result: CompleteResult = test().run(code)?;

    assert_eq!(result.exit_code, 1);
    assert_contains("invalid option", result.stderr);
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn exec_misc_values() -> Result {
    test()
        .run(r#"nu -n -c 'let x = "abc"; exec cococo $x ...[ a b c ]'"#)
        .expect_value_eq("abc a b c")
}

// `$nu.startup-time` starts inside the new program, so after `exec nu` it can't be longer than the
// time since just before the `exec`. Measured from process creation (macOS) or from the process's
// CPU time (Linux), it would also count the program that ran before `exec` and be longer. The
// comparison holds however fast or slow the machine is, so there is no time limit.
#[cfg(unix)]
#[test]
#[deps(NU)]
fn exec_restarts_startup_time() -> Result {
    let code = r#"
        nu -n --no-std-lib -c '
            $env.BEFORE_EXEC = date now | into int
            exec nu -n --no-std-lib -c "
                ($nu.startup-time | into int) <= (date now | into int) - ($env.BEFORE_EXEC | into int)
            "
        '
        | into bool
    "#;
    test().run(code).expect_value_eq(true)
}
