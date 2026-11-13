use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn print_to_stdout() -> Result {
    let actual: CompleteResult = test().run(r#"nu -n -c "print 'hello world'" | complete"#)?;
    assert_contains("hello world", actual.stdout);
    assert_eq!(actual.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn print_to_stderr() -> Result {
    let actual: CompleteResult = test().run(r#"nu -n -c "print -e 'hello world'" | complete"#)?;
    assert_eq!(actual.stdout, "");
    assert_contains("hello world", actual.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn print_raw() -> Result {
    let actual: CompleteResult = test().run("nu -n -c '0x[41 42 43] | print --raw' | complete")?;
    assert_eq!(actual.stdout, "ABC");
    Ok(())
}

#[test]
#[deps(NU)]
fn print_raw_stream() -> Result {
    let actual: CompleteResult =
        test().run("nu -n -c '[0x[66] 0x[6f 6f]] | bytes collect | print --raw' | complete")?;
    assert_eq!(actual.stdout, "foo");
    Ok(())
}
