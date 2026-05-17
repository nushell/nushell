use nu_test_support::prelude::*;

#[test]
fn export_command_help() -> Result {
    let actual: String = test().run("export -h")?;

    assert_contains(
        "Export definitions or environment variables from a module",
        actual,
    );

    Ok(())
}

#[test]
fn export_command_unexpected() -> Result {
    let err = test().run("export foo").expect_parse_error()?;
    match err {
        ParseError::UnexpectedKeyword(keyword, ..) => {
            assert_eq!(keyword, "export");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn export_alias_should_not_panic() -> Result {
    let err = test().run("export alias").expect_parse_error()?;
    match err {
        ParseError::UnknownState(msg, ..) => {
            assert_eq!(msg, "Missing positional after call check");
            Ok(())
        }
        err => Err(err.into()),
    }
}
