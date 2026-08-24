use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn view_source_returns_string() -> Result {
    let source = "def foo [] { echo hi }";
    let code = format!("{source}; view source foo");
    test().run(code).expect_value_eq(source)
}

#[test]
fn datasource_filepath_metadata(playground: Playground) -> Result {
    playground.file(
        "mdata.nu",
        indoc::indoc! {"
        def foo [] { echo hi }
    "},
    )?;

    let code = "
        source mdata.nu
        view source foo | metadata | get source
    ";

    let outcome: String = test().cwd(playground.path()).run(code)?;
    // expect path printed somehow
    assert_contains("mdata.nu", outcome);
    Ok(())
}
