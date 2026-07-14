use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn help() -> Result {
    let help: String = test().run("example one --help")?;
    assert_contains("test example 1", &help);
    assert_contains("Extra description for example one", &help);
    Ok(())
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn search_terms() -> Result {
    let code = r#"
        help commands
        | where name == "example one"
        | get 0.search_terms
    "#;

    test().run(code).expect_value_eq("example")
}
