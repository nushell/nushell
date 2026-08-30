use nu_test_support::prelude::*;

#[test]
fn returns_type_of_missing_file() -> Result {
    let code = r#"echo "spam.txt" | path type"#;
    test()
        .cwd("tests")
        .run(code)
        .expect_value_eq(Value::test_nothing())
}

#[test]
fn returns_type_of_existing_file(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
            echo "menu"
            | path type
        "#;

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("dir")
}

#[test]
fn returns_type_of_existing_directory(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
            echo "menu/spam.txt"
            | path type
        "#;

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("file")?;

    let code = r#"
            echo "~"
            | path type
        "#;

    test().run(code).expect_value_eq("dir")
}

#[test]
fn returns_type_of_existing_file_const(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
            const ty = ("menu" | path type);
            $ty
        "#;

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("dir")
}

#[test]
fn respects_cwd(playground: Playground) -> Result {
    playground.empty_file("foo/bar.txt")?;

    test()
        .cwd(playground.path())
        .run("cd foo; 'bar.txt' | path type")
        .expect_value_eq("file")
}
