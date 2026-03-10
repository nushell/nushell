use nu_test_support::prelude::*;

#[test]
fn concat_assign_list_int() -> Result {
    let code = r#"
        mut a = [1 2];
        $a ++= [3 4];
        $a == [1 2 3 4]
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn concat_assign_list_string() -> Result {
    let code = r#"
        mut a = [a b];
        $a ++= [c d];
        $a == [a b c d]
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn concat_assign_any() -> Result {
    let code = r#"
        mut a = [1 2 a];
        $a ++= [b 3];
        $a == [1 2 a b 3]
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn concat_assign_both_empty() -> Result {
    let code = r#"
        mut a = [];
        $a ++= [];
        $a == []
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn concat_assign_string() -> Result {
    let code = r#"
        mut a = 'hello';
        $a ++= ' world';
        $a == 'hello world'
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn concat_assign_type_mismatch() -> Result {
    let code = r#"
        mut a = [];
        $a ++= 'str'
    "#;

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::OperatorIncompatibleTypes { .. }));
    Ok(())
}

#[test]
fn concat_assign_runtime_type_mismatch() -> Result {
    let code = r#"
        mut a = [];
        $a ++= if true { 'str' }
    "#;

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::OperatorIncompatibleTypes { .. }));
    Ok(())
}
