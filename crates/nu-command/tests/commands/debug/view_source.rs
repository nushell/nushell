use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn view_source_returns_string() -> Result {
    let source = "def foo [] { echo hi }";
    let code = format!("{source}; view source foo");
    test().run(code).expect_value_eq(source)
}

#[test]
fn datasource_filepath_metadata() -> Result {
    Playground::setup("cd_ds_filepath_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "mdata.nu",
            r#"
                def foo [] { echo hi }
            "#,
        )]);

        let code = r#"
            source mdata.nu
            view source foo | metadata | get source
        "#;

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        // expect path printed somehow
        assert_contains("mdata.nu", outcome);
        Ok(())
    })
}
