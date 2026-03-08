use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[test]
fn returns_type_of_missing_file() -> Result {
    let code = r#"echo "spam.txt" | path type"#;
    test()
        .cwd("tests")
        .run(code)
        .expect_value_eq(Value::test_nothing())
}

#[test]
fn returns_type_of_existing_file() -> Result {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            echo "menu"
            | path type
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq("dir")
    })
}

#[test]
fn returns_type_of_existing_directory() -> Result {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            echo "menu/spam.txt"
            | path type
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq("file")?;

        let code = r#"
            echo "~"
            | path type
        "#;

        test().run(code).expect_value_eq("dir")
    })
}

#[test]
fn returns_type_of_existing_file_const() -> Result {
    Playground::setup("path_type_const", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let code = r#"
            const ty = ("menu" | path type);
            $ty
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq("dir")
    })
}

#[test]
fn respects_cwd() -> Result {
    Playground::setup("path_type_respects_cwd", |dirs, sandbox| {
        sandbox.within("foo").with_files(&[EmptyFile("bar.txt")]);

        test()
            .cwd(dirs.test())
            .run("cd foo; 'bar.txt' | path type")
            .expect_value_eq("file")
    })
}
