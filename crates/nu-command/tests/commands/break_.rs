use nu_test_support::prelude::*;

#[test]
fn break_for_loop() -> Result {
    let code = "
        mut vals = []
        for i in 1..10 { if $i == 3 { break }; $vals ++= [$i] }
        $vals
    ";

    test().run(code).expect_value_eq([1, 2])
}

#[test]
fn break_while_loop() -> Result {
    test()
        .run(r#"while true { break }; "hello""#)
        .expect_value_eq("hello")
}

#[test]
fn break_outside_loop() -> Result {
    let err = test().run("break").expect_compile_error()?;
    assert!(matches!(err, CompileError::NotInALoop { .. }));

    let err = test().run("do { break }").expect_compile_error()?;
    assert!(matches!(err, CompileError::NotInALoop { .. }));

    Ok(())
}
