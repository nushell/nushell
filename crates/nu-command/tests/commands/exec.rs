use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn basic_exec() -> Result {
    test()
        .run("nu -n -c 'exec nu --testbin cococo a b c'")
        .expect_value_eq("a b c")
}

#[test]
#[deps(NU)]
fn exec_complex_args() -> Result {
    test()
        .run("nu -n -c 'exec nu --testbin cococo b --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --'")
        .expect_value_eq("b --bar=2 -sab --arwr - -DTEEE=aasd-290 -90 --")
}

#[test]
#[deps(NU)]
fn exec_fail_batched_short_args() -> Result {
    let code = "
        nu -n -c 'exec nu --testbin cococo -ab 10'
        | complete
    ";
    let result: CompleteResult = test().run(code)?;

    assert_eq!(result.exit_code, 1);
    assert_contains("Unknown flag", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn exec_misc_values() -> Result {
    test()
        .run(r#"nu -n -c 'let x = "abc"; exec nu --testbin cococo $x ...[ a b c ]'"#)
        .expect_value_eq("abc a b c")
}
