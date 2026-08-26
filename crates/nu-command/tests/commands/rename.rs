use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

fn sample() -> Value {
    test_table![
        ["name"];
        ["Andrés N. Robalino"],
        ["JT Turner"],
        ["Yehuda Katz"],
        ["Jason Gedge"],
    ]
}

#[test]
fn changes_the_column_name() -> Result {
    let code = "$in | rename mosqueteros | get mosqueteros | length";

    test().run_with_data(code, sample()).expect_value_eq(4)
}

#[test]
fn keeps_remaining_original_names_given_less_new_names_than_total_original_names() -> Result {
    let code = "$in | default 'arepa!' hit | rename mosqueteros | get hit | length";

    test().run_with_data(code, sample()).expect_value_eq(4)
}

#[test]
fn errors_if_no_columns_present() -> Result {
    test()
        .run_with_data("$in.name | rename mosqueteros", sample())
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
fn errors_if_columns_param_is_empty() -> Result {
    let code = "$in | default 'arepa!' hit | rename --column {}";

    let err = test().run_with_data(code, sample()).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::TypeMismatch { err_message, .. }
            if err_message == "The column info cannot be empty"
    );
    Ok(())
}
