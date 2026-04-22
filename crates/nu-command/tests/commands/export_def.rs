use nu_test_support::prelude::*;

#[test]
fn export_subcommands_help() -> Result {
    let actual: String = test().run("export def -h")?;
    assert_contains(
        "Define a custom command and export it from a module",
        actual,
    );

    Ok(())
}

#[test]
fn export_should_not_expose_arguments() -> Result {
    // issue #16211
    let code = r#"
        export def foo [bar: int] {}
        scope variables | get name | "bar" in $in or "$bar" in $in
    "#;

    test().run(code).expect_value_eq(false)
}
