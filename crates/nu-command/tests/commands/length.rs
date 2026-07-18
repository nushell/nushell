use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::prelude::*;

#[test]
fn length_columns_in_cal_table() -> Result {
    test()
        .run("cal --as-table | columns | length")
        .expect_value_eq(7)
}

#[test]
fn length_columns_no_rows() -> Result {
    test().run("echo [] | length").expect_value_eq(0)
}

#[test]
fn length_fails_on_echo_record() -> Result {
    let err = test().run("echo {a:1 b:2} | length").expect_shell_error()?;

    match err {
        ShellError::OnlySupportsThisInputType {
            exp_input_type,
            wrong_type,
            ..
        } => {
            assert_eq!(
                exp_input_type,
                "list<any>, binary, nothing, and SQLiteQueryBuilder"
            );
            assert_eq!(wrong_type, "record<a: int, b: int>");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn length_byte_stream() {
    Playground::setup("length_bytes", |dirs, sandbox| {
        sandbox.mkdir("length_bytes");
        sandbox.with_files(&[FileWithContent("data.txt", "😀")]);

        let actual = nu!(cwd: dirs.test(), "open data.txt | length");
        assert_eq!(actual.out, "4");
    });
}
