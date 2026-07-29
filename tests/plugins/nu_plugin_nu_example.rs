use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn call() -> Result {
    let code = "
        (
            nu 
            --no-config-file 
            --no-std-lib
            --plugins crates/nu_plugin_nu_example/nu_plugin_nu_example.nu
            --commands 'nu_plugin_nu_example 4242 teststring'
        )
        | complete
    ";

    let CompleteResult {
        exit_code,
        stdout,
        stderr,
    } = test().run(code)?;
    let stdout = &stdout;
    let stderr = &stderr;

    assert_eq!(exit_code, 0);
    assert_contains("one", stdout);
    assert_contains("two", stdout);
    assert_contains("three", stdout);
    assert_contains("name: nu_plugin_nu_example", stderr);
    assert_contains("4242", stderr);
    assert_contains("teststring", stderr);
    Ok(())
}
