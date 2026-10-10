use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[cfg(feature = "sqlite")]
#[test]
fn can_query_single_table() -> Result {
    let code = r#"
        open sample.db
        | query db "select * from strings"
        | where x =~ ell
        | length
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(4)
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_sql_fails() -> Result {
    let code = r#"
        open sample.db
        | query db "select *asdfasdf"
    "#;

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::Generic(err) if err.error == "Failed to query SQLite database"
            && err.msg.contains("syntax error")
    );
    Ok(())
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_input_fails() -> Result {
    let err = test()
        .cwd("tests/fixtures/formats")
        .run(r#""foo" | query db "select * from asdf""#)
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "database" && from_type == "string"
    );
    Ok(())
}
