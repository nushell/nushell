use nu_test_support::prelude::*;

#[test]
fn early_return_if_true() -> Result {
    test()
        .run("def foo [x] { if true { return 2 }; $x }; foo 100")
        .expect_value_eq(2)
}

#[test]
fn early_return_if_false() -> Result {
    test()
        .run("def foo [x] { if false { return 2 }; $x }; foo 100")
        .expect_value_eq(100)
}

#[test]
#[deps(NU)]
fn return_works_in_script_without_def_main() -> Result {
    let actual: CompleteResult = test()
        .cwd("tests/fixtures/formats")
        .run("nu -n early_return.nu | complete")?;
    assert_eq!(actual.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn return_works_in_script_with_def_main() -> Result {
    let actual: CompleteResult = test()
        .cwd("tests/fixtures/formats")
        .run("nu -n early_return_outside_main.nu | complete")?;
    assert_eq!(actual.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn return_does_not_set_last_exit_code() -> Result {
    let code = "nu -n -c '
        hide-env LAST_EXIT_CODE;
        do --env { return 42 };
        $env.LAST_EXIT_CODE?
    ' | complete";

    let actual: CompleteResult = test().run(code)?;
    assert_eq!(actual.stdout.trim(), "");
    Ok(())
}
