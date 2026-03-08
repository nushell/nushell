use nu_test_support::prelude::*;

#[test]
fn splits_empty_path() -> Result {
    let code = r#"
        echo '' | path split | is-empty
    "#;

    let outcome: bool = test().cwd("tests").run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn splits_correctly_single_path() -> Result {
    let code = r#"
        'home/viking/spam.txt'
        | path split
        | last
    "#;

    test().cwd("tests").run(code).expect_value_eq("spam.txt")
}

#[test]
fn splits_correctly_single_path_const() -> Result {
    let code = r#"
        const result = ('home/viking/spam.txt' | path split);
        $result | last
    "#;

    test().run(code).expect_value_eq("spam.txt")
}
