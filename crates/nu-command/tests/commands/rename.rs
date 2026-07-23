use nu_test_support::prelude::*;

fn sample() -> serde_json::Value {
    json!([
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"]
    ])
}

#[test]
fn changes_the_column_name() -> Result {
    let code = "
        $in
        | wrap name
        | rename mosqueteros
        | get mosqueteros
        | length
    ";

    test().run_with_data(code, sample()).expect_value_eq(4)
}

#[test]
fn keeps_remaining_original_names_given_less_new_names_than_total_original_names() -> Result {
    let code = r#"
        $in
        | wrap name
        | default "arepa!" hit
        | rename mosqueteros
        | get hit
        | length
    "#;

    test().run_with_data(code, sample()).expect_value_eq(4)
}

#[test]
fn errors_if_no_columns_present() -> Result {
    let err = test()
        .run_with_data("$in | rename mosqueteros", sample())
        .expect_shell_error()?;

    match err {
        ShellError::OnlySupportsThisInputType {
            exp_input_type,
            wrong_type,
            ..
        } => {
            assert_eq!(exp_input_type, "record and table");
            assert_eq!(wrong_type, "list<list<string>>");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn errors_if_columns_param_is_empty() -> Result {
    let code = r#"
        $in
        | wrap name
        | default "arepa!" hit
        | rename --column {}
    "#;

    let err = test().run_with_data(code, sample()).expect_shell_error()?;

    match err {
        ShellError::TypeMismatch { err_message, .. } => {
            assert_eq!(err_message, "The column info cannot be empty");
            Ok(())
        }
        err => Err(err.into()),
    }
}
