use nu_test_support::prelude::*;

#[test]
fn each_while_preserves_closure_errors() -> Result {
    let err = test()
        .run(
            r#"[1 2 3] | each while {|x|
                if $x == 2 { error make {msg: "boom"} } else { $x }
            }"#,
        )
        .expect_shell_error()?;

    assert_contains("boom", format!("{err:?}"));
    Ok(())
}
