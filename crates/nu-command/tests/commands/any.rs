use nu_test_support::prelude::*;

#[test]
fn checks_any_row_is_true() -> Result {
    let code = r#"
        echo  [ "Ecuador", "USA", "New Zealand" ]
        | any {|it| $it == "New Zealand" }
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn checks_any_column_of_a_table_is_true() -> Result {
    let code = "
        echo [
                [  first_name, last_name,     rusty_at, likes  ];
                [      Andrés,  Robalino, '10/11/2013',   1    ]
                [          JT,    Turner, '10/12/2013',   1    ]
                [      Darren, Schroeder, '10/11/2013',   1    ]
                [      Yehuda,      Katz, '10/11/2013',   1    ]
        ]
        | any {|x| $x.rusty_at == '10/12/2013' }
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn checks_if_any_returns_error_with_invalid_command() -> Result {
    // Using `with-env` to remove `st` possibly being an external program
    let code = "
        [red orange yellow green blue purple]
        | any {|it| ($it | st length) > 4 }
    ";

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::ExternalCommand { label, help, .. } => {
            assert_eq!(label, "Command `st` not found");
            assert_eq!(help, "Did you mean `ast`?");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn works_with_1_param_blocks() -> Result {
    test()
        .run("[1 2 3] | any {|e| $e == 2 }")
        .expect_value_eq(true)
}

#[test]
fn works_with_0_param_blocks() -> Result {
    test()
        .run("[1 2 3] | any {|| $in == 2 }")
        .expect_value_eq(true)
}

#[test]
fn early_exits_with_1_param_blocks() -> Result {
    let code = r#"
        [1 2 3]
        | any {|e| if $e == 1 { true } else { error make {msg: "should not execute"} } }
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn early_exits_with_0_param_blocks() -> Result {
    let code = r#"
        [1 2 3]
        | any {|| if $in == 1 { true } else { error make {msg: "should not execute"} } }
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn any_uses_enumerate_index() -> Result {
    test()
        .run("[7 8 9] | enumerate | any {|el| $el.index == 2 }")
        .expect_value_eq(true)
}

#[test]
fn unique_env_each_iteration() -> Result {
    let code = r#"
        [1 2]
        | any {|| let ok = ($env.PWD | str ends-with 'formats'); cd '/'; if not $ok { error make {msg: "unexpected PWD"} }; false }
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(false)
}
