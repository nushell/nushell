use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
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
    test()
        .run("echo {a:1 b:2} | length")
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
fn length_propagates_error_stream() -> Result {
    let err = test()
        .run(r#"1..3 | each {|n| error make {msg: "x"} } | length"#)
        .expect_shell_error()?;

    assert_contains("x", format!("{err:?}"));
    Ok(())
}

#[test]
fn length_propagates_where_predicate_type_errors() -> Result {
    let err = test()
        .run("[[name size]; [a 100b] [b 200b]] | where size <= 150 | length")
        .expect_shell_error()?;

    assert_contains("compatible", format!("{err:?}"));
    Ok(())
}

#[test]
fn length_byte_stream() -> Result {
    Playground::setup("length_bytes", |dirs, sandbox| {
        sandbox.mkdir("length_bytes");
        sandbox.with_files(&[FileWithContent("data.txt", "😀")]);

        test()
            .cwd(dirs.test())
            .run("open data.txt | length")
            .expect_value_eq(4)
    })
}
