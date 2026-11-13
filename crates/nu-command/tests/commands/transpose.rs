use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn row() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: 1")));
    Ok(())
}

#[test]
fn row_but_last() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r -l | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: 2")));
    Ok(())
}

#[test]
fn row_but_all() -> Result {
    let actual: Vec<String> =
        test().run("[[key value]; [foo 1] [foo 2]] | transpose -r -a | debug")?;
    assert!(actual.iter().any(|row| row.contains("foo: [1, 2]")));
    Ok(())
}

#[test]
fn throw_inner_error() -> Result {
    let error_msg = "This message should show up";
    let error = format!("(error make {{ msg: \"{error_msg}\" }})");
    let err = test()
        .run(format!(
            "[[key value]; [foo 1] [foo 2] [{} 3]] | transpose",
            error
        ))
        .expect_shell_error()?;

    assert_contains(error_msg, err.to_string());
    Ok(())
}

#[test]
fn rejects_non_table_stream_input() -> Result {
    let err = test()
        .run("[1 2 3] | each { |it| ($it * 2) } | transpose | to nuon")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::OnlySupportsThisInputType { exp_input_type, .. }
            if exp_input_type == "table or record"
    );
    Ok(())
}
