use nu_protocol::{IntoPipelineData, PipelineMetadata, ShellError};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn get_index_then_split_chars() -> Result {
    // oh-my-style prompts: `($splits | get $x | split chars | get 0)` inside `each`.
    // `get` has a `(nothing, nothing)` pair; pipeline parsing unions input with
    // `nothing`. That must not make `get` output `nothing`, or `split chars` is
    // inferred as error input.
    let code = r#"
        let splits = "a/b/c" | split row "/"
        1..<2 | each {|x|
            ($splits | get $x | split chars | get 0)
        } | get 0
    "#;
    test().run(code).expect_value_eq("b")
}

#[test]
fn simple_get_record() -> Result {
    test()
        .run_with_data("$in | get foo", test_value!({ foo: "bar" }))
        .expect_value_eq("bar")
}

#[test]
fn simple_get_list() -> Result {
    test()
        .run_with_data("$in | get foo", test_value!([{ foo: "bar" }]))
        .expect_value_eq(["bar"])
}

#[test]
fn fetches_a_row() -> Result {
    test()
        .run_with_data(
            "$in | get nu_party_venue",
            test_value!({ nu_party_venue: "zion" }),
        )
        .expect_value_eq("zion")
}

#[test]
fn fetches_by_index() -> Result {
    let data = test_value!({
        package: {
            name: "nu",
            version: "0.4.1",
            authors: [
                "Yehuda Katz <wycats@gmail.com>",
                "JT Turner <547158+jntrnr@users.noreply.github.com>",
                "Andrés N. Robalino <andres@androbtech.com>",
            ],
            description: "When arepas shells are tasty and fun.",
        }
    });

    test()
        .run_with_data("$in | get package.authors.2", data)
        .expect_value_eq("Andrés N. Robalino <andres@androbtech.com>")
}

#[test]
fn fetches_by_column_path() -> Result {
    test()
        .run_with_data(
            "$in | get package.name",
            test_value!({ package: { name: "nu" } }),
        )
        .expect_value_eq("nu")
}

#[test]
fn column_paths_are_either_double_quoted_or_regular_unquoted_words_separated_by_dot() -> Result {
    let data = test_value!({
        package: {
            "9999": [
                "Yehuda Katz <wycats@gmail.com>",
                "JT Turner <jtd.turner@gmail.com>",
                "Andrés N. Robalino <andres@androbtech.com>",
            ],
            description: "When arepas shells are tasty and fun.",
        }
    });

    test()
        .run_with_data(r#"$in | get package."9999" | length"#, data)
        .expect_value_eq(3)
}

#[test]
fn fetches_more_than_one_column_path() -> Result {
    let data = test_value!({
        fortune_tellers: [
            { name: "Andrés N. Robalino", arepas: 1 },
            { name: "JT", arepas: 1 },
            { name: "Yehuda Katz", arepas: 1 },
        ]
    });

    let code = "
        $in
        | get fortune_tellers.2.name fortune_tellers.0.name fortune_tellers.1.name
        | get 2
    ";

    test().run_with_data(code, data).expect_value_eq("JT")
}

#[test]
fn fetches_columns_with_literal_list_spread() -> Result {
    test()
        .run("[{a: 1, b: 2, c: 3}] | get ...[a c]")
        .expect_value_eq(test_value!([[1], [3]]))
}

#[test]
fn fetches_columns_with_variable_list_spread() -> Result {
    test()
        .run("let cols = [a c]; [{a: 1, b: 2, c: 3}] | get ...$cols")
        .expect_value_eq(test_value!([[1], [3]]))
}

#[test]
fn no_cell_path_returns_input_unchanged() -> Result {
    test()
        .run_with_data("$in | get", test_value!([1, 2, 3]))
        .expect_value_eq([1, 2, 3])
}

#[test]
fn errors_fetching_by_column_not_present() -> Result {
    let data = test_value!({
        tacos: { sentence_words: ["Yo", "quiero", "tacos"] },
        pizzanushell: { "sentence-words": ["I", "want", "pizza"] },
    });

    let err = test()
        .run_with_data("$in | get taco", data)
        .expect_shell_error()?;
    assert_matches!(err, ShellError::DidYouMean { suggestion, .. } if suggestion == "tacos");
    Ok(())
}

#[test]
fn errors_fetching_by_column_using_a_number() -> Result {
    let data = test_value!({
        spanish_lesson: { "0": "can only be fetched with 0 double quoted." }
    });

    let err = test()
        .run_with_data("$in | get spanish_lesson.0", data)
        .expect_shell_error()?;
    assert_matches!(err, ShellError::TypeMismatch { .. });
    Ok(())
}

#[test]
fn errors_fetching_by_index_out_of_bounds() -> Result {
    let data = test_value!({
        spanish_lesson: { sentence_words: ["Yo", "quiero", "taconushell"] }
    });

    let err = test()
        .run_with_data("$in | get spanish_lesson.sentence_words.3", data)
        .expect_shell_error()?;
    assert_matches!(err, ShellError::AccessBeyondEnd { max_idx: 2, .. });
    Ok(())
}

#[test]
fn errors_fetching_by_accessing_empty_list() -> Result {
    let err = test()
        .run_with_data("$in | get 3", test_value!([]))
        .expect_shell_error()?;
    assert_matches!(err, ShellError::AccessEmptyContent { .. });
    Ok(())
}

#[test]
fn quoted_column_access() -> Result {
    test()
        .run_with_data(
            r#"$in | get "foo bar".baz.0"#,
            test_value!([{ "foo bar": { baz: 4 } }]),
        )
        .expect_value_eq(4)
}

#[test]
fn get_does_not_delve_too_deep_in_nested_lists() -> Result {
    let err = test()
        .run_with_data("$in | get foo", test_value!([[{ foo: "bar" }]]))
        .expect_shell_error()?;
    assert_matches!(err, ShellError::CantFindColumn { col_name, .. } if col_name == "foo");
    Ok(())
}

#[test]
fn ignore_errors_works() -> Result {
    test()
        .run_with_data(r#"let path = "foo"; $in | get -o $path"#, test_value!({}))
        .expect_value_eq(())
}

#[test]
fn ignore_multiple() -> Result {
    test()
        .run_with_data("$in | get -o c d", test_table![["a"]; ["b"]])
        .expect_value_eq(test_value!([[()], [()]]))
}

#[test]
fn test_const() -> Result {
    test()
        .run("const x = [1 2 3] | get 1; $x")
        .expect_value_eq(2)
}

#[test]
fn get_with_negative_number_reports_clear_error() -> Result {
    let err = test()
        .run_with_data("$in | get (-2)", test_value!([1, 2, 3]))
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "cell path" && from_type == "negative number"
    );
    Ok(())
}

#[test]
fn test_const_with_no_cell_path() -> Result {
    test()
        .run("const x = [1 2 3] | get; $x")
        .expect_value_eq([1, 2, 3])
}

enum Metadata {
    Keep,
    Drop,
}

#[rstest]
#[case::no_cell_path("get", Metadata::Keep)]
#[case::index_only("get 1", Metadata::Keep)]
#[case::two_indices("get 1 0", Metadata::Keep)]
#[case::index_and_column("get name 1", Metadata::Keep)]
#[case::single_column("get name", Metadata::Drop)]
#[case::cellpath_with_multiple_members("get 1.name", Metadata::Drop)]
#[case::multiple_columns("get name type", Metadata::Drop)]
fn test_path_columns_metadata(#[case] code: &str, #[case] metadata: Metadata) -> Result {
    let in_metadata = Some(PipelineMetadata::default().with_path_columns(vec!["name".into()]));

    let data = test_value!([
        { name: "Cargo.toml", type: "file" },
        { name: "src", type: "dir" },
    ])
    .into_pipeline_data_with_metadata(in_metadata.clone());

    let out_metadata = test().run_raw_with_data(code, data)?.body.take_metadata();

    let target_metadata = match metadata {
        Metadata::Keep => in_metadata,
        Metadata::Drop => Some(PipelineMetadata::default()),
    };

    assert_eq!(target_metadata, out_metadata);
    Ok(())
}
