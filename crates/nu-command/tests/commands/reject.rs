use nu_protocol::{Filesize, IntoPipelineData, PipelineMetadata, test_record};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn regular_columns() -> Result {
    let code = "
        $in
        | reject type first_name
        | columns
    ";

    let input = test_table![
        ["first_name", "last_name", "rusty_at", "type"];
        ["Andres", "Robalino", "10/11/2013", "A"],
        ["JT", "Turner", "10/12/2013", "B"],
        ["Yehuda", "Katz", "10/11/2013", "A"],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(["last_name", "rusty_at"])
}

#[test]
fn skip_cell_rejection() -> Result {
    let code = "$in | reject a | get c?.0";
    let input = test_value!([{ a: 1, b: 2, c: "txt" }, { a: "val" }]);

    test().run_with_data(code, input).expect_value_eq("txt")
}

#[test]
fn complex_nested_columns() -> Result {
    let code = r#"
        $in
        | reject nu."0xATYKARNU" nu.committers
        | get nu
        | columns
    "#;

    let input = test_value!({
        nu: {
            committers: [
                { name: "Andres N. Robalino" },
                { name: "JT Turner" },
                { name: "Yehuda Katz" },
            ],
            releases: [
                { version: "0.2" },
                { version: "0.8" },
                { version: "0.9999999" },
            ],
            "0xATYKARNU": [
                ["Th", "e", " "],
                ["BIG", " ", "UnO"],
                ["punto", "cero"],
            ],
        },
    });

    test()
        .run_with_data(code, input)
        .expect_value_eq(["releases"])
}

#[test]
fn ignores_duplicate_columns_rejected() -> Result {
    let code = r#"
        $in
        | reject "first name" "first name"
        | columns
    "#;

    let input = test_table![
        ["first name", "last name"];
        ["Andres", "Robalino"],
        ["Andres", "Jnth"],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(["last name"])
}

#[test]
fn ignores_duplicate_rows_rejected() -> Result {
    let code = "$in | reject 2 2";
    let input = test_table![
        ["a", "b"];
        [1, 2],
        [3, 4],
        [5, 6],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["a", "b"];
            [1, 2],
            [3, 4],
        ])
}

#[test]
fn reject_record_from_raw_eval() -> Result {
    let code = "$in | reject a";
    let input = test_value!({ a: 3 });

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_value!({}))
}

#[test]
fn reject_table_from_raw_eval() -> Result {
    let code = "$in | reject a";
    let input = test_value!([{ a: 3 }]);

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_value!([{}]))
}

#[test]
fn reject_nested_field() -> Result {
    let code = "$in | reject a.b";
    let input = test_value!({ a: { b: 3, c: 5 } });

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_value!({ a: { c: 5 } }))
}

#[rstest]
#[case::record(test_value!({}), test_value!({}))]
#[case::list_missing_column(test_value!([{}]), test_value!([{}]))]
#[case::list_some_missing(test_value!([{}, { foo: 2 }]), test_value!([{}, {}]))]
#[case::list_all_present(test_value!([{ foo: 1 }, { foo: 2 }]), test_value!([{}, {}]))]
fn reject_optional_column(#[case] input: Value, #[case] expected: Value) -> Result {
    let code = "$in | reject foo?";

    test().run_with_data(code, input).expect_value_eq(expected)
}

#[test]
fn reject_optional_row() -> Result {
    let code = "$in | reject 3?";
    let input = test_table![
        ["foo"];
        ["bar"],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["foo"];
            ["bar"],
        ])
}

#[test]
fn reject_columns_with_list_spread() -> Result {
    let code = "let arg = [type size]; $in | reject ...$arg";
    let input = test_table![
        ["name", "type", "size"];
        ["Cargo.toml", "file", Filesize::from(10_000_000)],
        ["Cargo.lock", "file", Filesize::from(10_000_000)],
        ["src", "dir", Filesize::from(100_000_000)],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["name"];
            ["Cargo.toml"],
            ["Cargo.lock"],
            ["src"],
        ])
}

#[test]
fn reject_rows_with_list_spread() -> Result {
    let code = "let arg = [2 0]; $in | reject ...$arg";
    let input = test_table![
        ["name", "type", "size"];
        ["Cargo.toml", "file", Filesize::from(10_000_000)],
        ["Cargo.lock", "file", Filesize::from(10_000_000)],
        ["src", "dir", Filesize::from(100_000_000)],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["name", "type", "size"];
            ["Cargo.lock", "file", Filesize::from(10_000_000)],
        ])
}

#[test]
fn reject_mixed_with_list_spread() -> Result {
    let code = "let arg = [type 2]; $in | reject ...$arg";
    let input = test_table![
        ["name", "type", "size"];
        ["Cargp.toml", "file", Filesize::from(10_000_000)],
        ["Cargo.lock", "file", Filesize::from(10_000_000)],
        ["src", "dir", Filesize::from(100_000_000)],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["name", "size"];
            ["Cargp.toml", Filesize::from(10_000_000)],
            ["Cargo.lock", Filesize::from(10_000_000)],
        ])
}

#[test]
fn reject_multiple_rows_ascending() -> Result {
    let code = "$in | reject 1 2";
    let input = test_table![
        ["a", "b"];
        [1, 2],
        [3, 4],
        [5, 6],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["a", "b"];
            [1, 2],
        ])
}

#[test]
fn reject_multiple_rows_descending() -> Result {
    let code = "$in | reject 2 1";
    let input = test_table![
        ["a", "b"];
        [1, 2],
        [3, 4],
        [5, 6],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["a", "b"];
            [1, 2],
        ])
}

#[test]
fn test_ignore_errors_flag() -> Result {
    let code = "$in | reject 5 -o";
    let input = test_table![
        ["a", "b"];
        [1, 2],
        [3, 4],
        [5, 6],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["a", "b"];
            [1, 2],
            [3, 4],
            [5, 6],
        ])
}

#[test]
fn test_ignore_errors_flag_var() -> Result {
    let code = "let arg = [5 c]; $in | reject ...$arg -o";
    let input = test_table![
        ["a", "b"];
        [1, 2],
        [3, 4],
        [5, 6],
    ];

    test()
        .run_with_data(code, input)
        .expect_value_eq(test_table![
            ["a", "b"];
            [1, 2],
            [3, 4],
            [5, 6],
        ])
}

#[test]
fn test_works_with_integer_path_and_stream() -> Result {
    let code = "$in | reject 1";
    let input = test_value!(["N", "u", "s", "h", "e", "l", "l"]);

    test()
        .run_with_data(code, input)
        .expect_value_eq(["N", "s", "h", "e", "l", "l"])
}

enum ExpectTo {
    Keep,
    Drop,
}

#[rstest]
#[case::index_only("reject 1", ExpectTo::Keep)]
#[case::two_indices("reject 1 0", ExpectTo::Keep)]
#[case::index_and_column("reject type 1", ExpectTo::Keep)]
#[case::single_column("reject name", ExpectTo::Drop)]
#[case::case_insensitive("reject NaMe!", ExpectTo::Drop)]
#[case::multiple_columns("reject name type", ExpectTo::Drop)]
fn test_path_columns_metadata(#[case] code: &str, #[case] expect_to: ExpectTo) -> Result {
    let in_metadata = Some(
        PipelineMetadata::default()
            .with_path_columns(vec!["name".into()])
            .with_content_type(Some("text/palin".into())),
    );

    let data = Value::test_list(vec![
        test_record! { "name" => "Cargo.toml", "type" => "file" },
        test_record! { "name" => "src",        "type" => "dir" },
    ])
    .into_pipeline_data_with_metadata(in_metadata.clone());

    let out_metadata = test().run_raw_with_data(code, data)?.body.take_metadata();

    let target_metadata = match expect_to {
        ExpectTo::Keep => in_metadata,
        ExpectTo::Drop => in_metadata.map(|m| m.with_path_columns(vec![])),
    };

    assert_eq!(target_metadata, out_metadata);
    Ok(())
}

#[test]
fn forwards_error_properly() -> Result {
    let err = test()
        .run("ls | insert foo { error make { msg: 'boo' } } | reject name")
        .expect_error()?;

    assert_eq!(&err.into_labeled()?.msg, "boo");

    Ok(())
}

#[test]
fn reject_with_negative_index_reports_clear_error() -> Result {
    let err = test().run("[1 2 3] | reject (-2)").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "cell path" && from_type == "negative number"
    );
    Ok(())
}
