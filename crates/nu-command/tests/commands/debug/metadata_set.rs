use std::collections::HashMap;

use nu_test_support::prelude::*;

#[test]
fn errors_on_conflicting_metadata_flags() -> Result {
    let code = r#"
        echo "foo" 
        | metadata set --datasource-filepath foo.txt --datasource-ls
    "#;

    let err = test().run(code).expect_error()?;
    match err {
        ShellError::IncompatibleParameters {
            left_message,
            right_message,
            ..
        } => {
            assert_eq!(left_message, "cannot use `--datasource-filepath`");
            assert_eq!(right_message, "with `--datasource-ls`");
            Ok(())
        }
        _ => Err(err.into()),
    }
}

#[test]
fn works_with_datasource_filepath() -> Result {
    let code = r#"
    echo "foo"
    | metadata set --datasource-filepath foo.txt
    | metadata
    "#;

    let outcome: HashMap<String, Value> = test().run(code)?;
    assert_eq!(outcome["source"].as_str()?, "foo.txt");
    Ok(())
}

#[test]
fn works_with_datasource_ls() -> Result {
    let code = r#"
        echo "foo"
        | metadata set --datasource-ls
        | metadata
    "#;

    let outcome: HashMap<String, Value> = test().run(code)?;
    assert_eq!(outcome["source"].as_str()?, "ls");
    Ok(())
}

#[test]
fn works_with_path_columns_single() -> Result {
    let code = "[] | metadata set --path-columns [test] | metadata | get path_columns.0";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "test");
    Ok(())
}

#[test]
fn works_with_path_columns_multiple() -> Result {
    let code = r#"[] | metadata set --path-columns [name path] | metadata | get path_columns | str join " ""#;
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "name path");
    Ok(())
}

#[test]
fn works_with_merge_arbitrary_metadata() -> Result {
    let code = r#"
        echo "foo"
        | metadata set { merge {custom_key: "custom_value", foo: 42} }
        | metadata
        | get custom_key
        "#;

    let outcome: String = test().cwd(".").run(code)?;
    assert_eq!(outcome, "custom_value");
    Ok(())
}

#[test]
fn merge_preserves_existing_metadata() -> Result {
    let code = r#"
        echo "foo"
        | metadata set --content-type "text/plain"
        | metadata set { merge {custom: "value"} }
        | metadata
        | get content_type
        "#;

    let outcome: String = test().cwd(".").run(code)?;
    assert_eq!(outcome, "text/plain");
    Ok(())
}

#[test]
fn custom_metadata_preserved_through_collect() -> Result {
    let code = r#"
        echo "foo"
        | metadata set { merge {custom_key: "custom_value"} }
        | collect
        | metadata
        | get custom_key
        "#;

    let outcome: String = test().cwd(".").run(code)?;
    assert_eq!(outcome, "custom_value");
    Ok(())
}

#[test]
fn closure_adds_custom_without_clobbering_existing() -> Result {
    let code = r#"
        "data" 
        | metadata set --content-type "text/csv" 
        | metadata set {|m| $m | upsert custom_key "value"} 
        | metadata
    "#;

    #[derive(Debug, FromValue)]
    struct Outcome {
        content_type: String,
        custom_key: String,
    }

    let outcome: Outcome = test().run(code)?;
    assert_eq!(outcome.content_type, "text/csv");
    assert_eq!(outcome.custom_key, "value");
    Ok(())
}

#[test]
fn errors_when_closure_with_flags() -> Result {
    let code = r#"
        echo "foo" | metadata set {|| {content_type: "text/plain"}} --content-type "ignored"
    "#;
    let err = test().run(code).expect_error()?;
    let msg = err.generic_msg()?;
    assert_eq!(msg, "cannot use closure with other flags");
    Ok(())
}

#[test]
fn errors_when_closure_returns_non_record() -> Result {
    let code = r#"
    echo "foo" | metadata set {|meta| "not a record"}
    "#;

    let err = test().run(code).expect_error()?;
    assert_contains("Closure must return a record", err.to_string());
    Ok(())
}
