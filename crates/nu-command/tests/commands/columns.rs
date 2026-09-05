use nu_test_support::prelude::*;

#[test]
fn columns_propagates_error_stream() -> Result {
    let err = test()
        .run(r#"1..3 | each {|n| error make {msg: "x"} } | columns"#)
        .expect_shell_error()?;

    assert_contains("x", format!("{err:?}"));
    Ok(())
}

#[test]
fn columns_propagates_where_predicate_type_errors() -> Result {
    let err = test()
        .run("[[name size]; [a 100b] [b 200b]] | where size <= 150 | columns")
        .expect_shell_error()?;

    assert_contains("compatible", format!("{err:?}"));
    Ok(())
}
