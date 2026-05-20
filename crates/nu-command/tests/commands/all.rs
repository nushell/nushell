use nu_test_support::prelude::*;

#[test]
fn checks_all_rows_are_true() -> Result {
    let code = r#"
        echo  [ "Andrés", "Andrés", "Andrés" ]
        | all {|it| $it == "Andrés" }
    "#;

    test().run(code).expect_value_eq(true)
}

#[test]
fn checks_all_rows_are_false_with_param() -> Result {
    test()
        .run("[1, 2, 3, 4] | all { |a| $a >= 5 }")
        .expect_value_eq(false)
}

#[test]
fn checks_all_rows_are_true_with_param() -> Result {
    test()
        .run("[1, 2, 3, 4] | all { |a| $a < 5 }")
        .expect_value_eq(true)
}

#[test]
fn checks_all_columns_of_a_table_is_true() -> Result {
    let code = "
        echo [
                [  first_name, last_name,     rusty_at, likes  ];
                [      Andrés,  Robalino, '10/11/2013',   1    ]
                [          JT,    Turner, '10/12/2013',   1    ]
                [      Darren, Schroeder, '10/11/2013',   1    ]
                [      Yehuda,      Katz, '10/11/2013',   1    ]
        ]
        | all {|x| $x.likes > 0 }
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn checks_if_all_returns_error_with_invalid_command() -> Result {
    let code = "
        [red orange yellow green blue purple] 
        | all {|it| ($it | st length) > 4 }
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
        .run("[1 2 3] | all {|e| $e in [1 2 3] }")
        .expect_value_eq(true)
}

#[test]
fn works_with_0_param_blocks() -> Result {
    test()
        .run("[1 2 3] | all {|| $in in [1 2 3] }")
        .expect_value_eq(true)
}

#[test]
fn early_exits_with_1_param_blocks() -> Result {
    let code = r#"
        [1 2 3]
        | all {|e| if $e == 1 { false } else { error make {msg: "should not execute"} } }
    "#;

    test().run(code).expect_value_eq(false)
}

#[test]
fn early_exits_with_0_param_blocks() -> Result {
    let code = r#"
        [1 2 3]
        | all {|| if $in == 1 { false } else { error make {msg: "should not execute"} } }
    "#;

    test().run(code).expect_value_eq(false)
}

#[test]
fn all_uses_enumerate_index() -> Result {
    test()
        .run("[7 8 9] | enumerate | all {|el| $el.index < 3 }")
        .expect_value_eq(true)
}

#[test]
fn unique_env_each_iteration() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("[1 2] | all {|| let ok = ($env.PWD | str ends-with 'formats'); cd '/'; $ok }")
        .expect_value_eq(true)
}
