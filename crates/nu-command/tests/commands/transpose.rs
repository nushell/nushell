use nu_protocol::test_record;
use nu_test_support::prelude::*;

#[test]
fn row() -> Result {
    test()
        .run("[[key value]; [foo 1] [foo 2]] | transpose -r")
        .expect_value_eq([test_record! { "foo" => 1 }])
}

#[test]
fn row_but_last() -> Result {
    test()
        .run("[[key value]; [foo 1] [foo 2]] | transpose -r -l")
        .expect_value_eq([test_record! { "foo" => 2 }])
}

#[test]
fn row_but_all() -> Result {
    test()
        .run("[[key value]; [foo 1] [foo 2]] | transpose -r -a")
        .expect_value_eq([test_record! { "foo" => [1, 2] }])
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

    assert_eq!(err.into_labeled()?.msg, error_msg);
    Ok(())
}

#[test]
fn rejects_non_table_stream_input() -> Result {
    let err = test()
        .run("[1 2 3] | each { |it| ($it * 2) } | transpose | to nuon")
        .expect_shell_error()?;

    match err {
        ShellError::OnlySupportsThisInputType {
            exp_input_type,
            wrong_type,
            ..
        } => {
            assert_eq!(exp_input_type, "table or record");
            assert_eq!(wrong_type, "list<any>");
            Ok(())
        }
        err => Err(err.into()),
    }
}
