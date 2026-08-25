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
<<<<<<< HEAD
        echo "menu"
        | path type
    "#;
=======
            echo "menu"
            | path type
        "#;
>>>>>>> 1832dd598 (modernized more tests)

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("dir")
}

#[test]
fn returns_type_of_existing_directory(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
<<<<<<< HEAD
        echo "menu/spam.txt"
        | path type
    "#;
=======
            echo "menu/spam.txt"
            | path type
        "#;
>>>>>>> 1832dd598 (modernized more tests)

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("file")?;

    let code = r#"
<<<<<<< HEAD
        echo "~"
        | path type
    "#;
=======
            echo "~"
            | path type
        "#;
>>>>>>> 1832dd598 (modernized more tests)

    test().run(code).expect_value_eq("dir")
}

#[test]
fn returns_type_of_existing_file_const(playground: Playground) -> Result {
    playground.empty_file("menu/spam.txt")?;

    let code = r#"
<<<<<<< HEAD
        const ty = ("menu" | path type);
        $ty
    "#;
=======
            const ty = ("menu" | path type);
            $ty
        "#;
>>>>>>> 1832dd598 (modernized more tests)

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
