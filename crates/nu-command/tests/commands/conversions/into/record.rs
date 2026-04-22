use nu_test_support::prelude::*;

#[test]
fn doesnt_accept_mixed_type_list_as_input() -> Result {
    let code = "[{foo: bar} [quux baz]] | into record";
    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::TypeMismatch { .. }));
    Ok(())
}
