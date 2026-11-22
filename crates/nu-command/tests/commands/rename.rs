use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

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

#[test]
fn reports_missing_column_in_later_record() -> Result {
    let mut tester = test();
    let err = tester
        .run("[{name: Alice} {age: 30}] | rename --column {name: n}")
        .expect_shell_error()?;
    let ShellError::UnsupportedInput {
        msg,
        input,
        msg_span,
        input_span,
    } = err
    else {
        panic!("expected an unsupported input error, got {err:?}");
    };
    assert_eq!(msg, "cannot rename column 'name'");
    assert_eq!(input, "row 1 is missing column 'name'");
    assert_eq!(tester.engine_state.get_span_contents(msg_span), b"rename");
    assert_eq!(
        tester.engine_state.get_span_contents(input_span),
        b"{age: 30}"
    );
    Ok(())
}

#[rstest]
#[case::json_array(r#"'[{"name":"a"},{"name":"b"},{"age":1}]' | from json"#)]
#[case::json_stream(
    r#"'{"name":"a"}
{"name":"b"}
{"age":1}' | lines | each { from json }"#
)]
fn reports_missing_column_with_shared_source_span(#[case] source: &str) -> Result {
    let err = test()
        .run(format!("{source} | rename --column {{name: n}}"))
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, input, .. }
            if msg == "cannot rename column 'name'" && input == "row 2 is missing column 'name'"
    );
    Ok(())
}

#[test]
fn reports_missing_column_in_single_record() -> Result {
    let err = test()
        .run("{age: 30} | rename --column {name: n}")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, input, .. }
            if msg == "cannot rename column 'name'" && input == "this record is missing column 'name'"
    );
    Ok(())
}

#[test]
fn renames_mapped_column_in_all_records() -> Result {
    test()
        .run("[[name age]; [Alice 30] [Bob 40]] | rename --column {name: n}")
        .expect_value_eq(test_table![["n", "age"]; ["Alice", 30], ["Bob", 40]])
}
