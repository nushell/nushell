use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn inspect_with_empty_pipeline() -> Result {
    let err = test().run("inspect").expect_shell_error()?;
    assert_matches!(err, ShellError::PipelineEmpty { .. });
    Ok(())
}

#[test]
#[deps(NU)]
fn inspect_with_empty_list() -> Result {
    let actual: CompleteResult = test().run("nu -n -c '[] | inspect' | complete")?;
    assert_contains("empty list", actual.stdout);
    Ok(())
}

#[test]
#[deps(NU)]
fn inspect_with_empty_record() -> Result {
    let actual: CompleteResult = test().run("nu -n -c '{} | inspect' | complete")?;
    assert_contains("empty record", actual.stdout);
    Ok(())
}
