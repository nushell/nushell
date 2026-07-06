use nu_test_support::prelude::*;

#[test]
#[deps(NU)]
fn interleave_external_commands() -> Result {
    let code = r#"
        (
            interleave
            {
                nu -n -c 'print hello; print world'
                | lines
                | each { 'greeter: ' ++ $in }
            }
            {
                nu -n -c 'print nushell; print rocks'
                | lines
                | each { 'evangelist: ' ++ $in }
            }
        )
    "#;
    let out: Vec<String> = test().run(code)?;
    let out = out.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let out = &out;
    assert_contains("greeter: hello", out);
    assert_contains("greeter: world", out);
    assert_contains("evangelist: nushell", out);
    assert_contains("evangelist: rocks", out);

    Ok(())
}
