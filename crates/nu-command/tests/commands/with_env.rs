use nu_test_support::prelude::*;

#[test]
fn with_env_extends_environment() -> Result {
    test()
        .run("with-env { FOO: BARRRR } {echo $env} | get FOO")
        .expect_value_eq("BARRRR")
}

#[test]
fn with_env_shorthand() -> Result {
    test()
        .run("FOO=BARRRR echo $env | get FOO")
        .expect_value_eq("BARRRR")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn shorthand_doesnt_reorder_arguments() -> Result {
    test()
        .run("FOO=BARRRR cococo first second")
        .expect_value_eq("first second")
}

#[test]
fn with_env_shorthand_trims_quotes() -> Result {
    test()
        .run("FOO='BARRRR' echo $env | get FOO")
        .expect_value_eq("BARRRR")
}

#[test]
fn with_env_and_shorthand_same_result() -> Result {
    let actual_shorthand: String = test().run("FOO='BARRRR' echo $env | get FOO")?;
    let actual_normal: String = test().run("with-env { FOO: BARRRR } {echo $env} | get FOO")?;

    assert_eq!(actual_shorthand, actual_normal);
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn test_redirection2() -> Result {
    test()
        .run("let x = (FOO=BAR cococo niceenvvar); $x | str trim | str length")
        .expect_value_eq(10)
}

#[test]
fn with_env_hides_variables_in_parent_scope() -> Result {
    let code = r#"
        $env.FOO = "1"
        let before = $env.FOO
        let during = (with-env { FOO: null } { $env.FOO })
        let after = $env.FOO
        [$before $during $after]
    "#;

    test().run(code).expect_value_eq(("1", (), "1"))
}

#[test]
fn with_env_shorthand_can_not_hide_variables() -> Result {
    let code = r#"
        $env.FOO = "1"
        let before = $env.FOO
        let during = (FOO=null do { $env.FOO })
        let after = $env.FOO
        [$before $during $after]
    "#;

    test().run(code).expect_value_eq(("1", "null", "1"))
}
