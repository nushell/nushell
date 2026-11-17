use indoc::indoc;
use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_omits_labels_and_footers(
    #[case] mode: &str,
    #[values(false, true)] header_on_separator: bool,
) -> Result {
    test()
        .run_with_data(
            format!(
                "let data = $in
                $env.config.footer_mode = 'always'
                $env.config.table.header_on_separator = {header_on_separator}
                $data | table {mode} --no-headers --index false --theme basic --width 80"
            ),
            test_table![
                ["long_first_label", "long_second_label"];
                [11, 22],
                [33, 44],
            ],
        )
        .expect_value_eq(indoc! {"
            +----+----+
            | 11 | 22 |
            +----+----+
            | 33 | 44 |
            +----+----+
        "})
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_preserves_values_identical_to_labels(#[case] mode: &str) -> Result {
    test()
        .run_with_data(
            format!("table {mode} --no-headers --index false --theme basic"),
            test_table![["same"]; ["same"], ["tail"]],
        )
        .expect_value_eq(indoc! {"
            +------+
            | same |
            +------+
            | tail |
            +------+
        "})
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_preserves_custom_indexes(#[case] mode: &str) -> Result {
    test()
        .run_with_data(
            format!("table {mode} --no-headers --index true --theme basic"),
            test_table![["index", "label"]; ["row_a", 11], ["row_b", 22]],
        )
        .expect_value_eq(indoc! {"
            +-------+----+
            | row_a | 11 |
            +-------+----+
            | row_b | 22 |
            +-------+----+
        "})
}

#[rstest]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_applies_to_nested_tables_but_preserves_record_keys(#[case] mode: &str) -> Result {
    let actual: String = test().run_with_data(
        format!("table {mode} --no-headers --index false --width 120"),
        test_value!([{
            outer_label: {
                record_key: [{ inner_label: "first_value" }, { inner_label: "last_value" }],
            },
        }]),
    )?;
    for label in ["outer_label", "inner_label"] {
        assert_contains_not(label, &actual);
    }
    for value in ["record_key", "first_value", "last_value"] {
        assert_eq!(actual.matches(value).count(), 1, "{actual}");
    }
    Ok(())
}

#[rstest]
#[case::record("{record_key: 42}")]
#[case::list("[11 22]")]
#[case::empty_list("[]")]
#[case::empty_record("{}")]
#[case::mixed_list("[{record_key: 11} plain_value]")]
fn no_headers_preserves_layouts_without_column_headers(#[case] input: &str) -> Result {
    let mut tester = test();
    for mode in ["", "--expand", "--collapse"] {
        let expected: String = tester.run(format!("{input} | table {mode}"))?;
        tester
            .run(format!("{input} | table {mode} --no-headers"))
            .expect_value_eq(expected)?;
    }
    Ok(())
}

#[test]
fn no_headers_preserves_heterogeneous_collapsed_record_keys() -> Result {
    let input = "[{left_key: 11} {right_key: 22}]";
    let mut tester = test();
    let expected: String = tester.run(format!("{input} | table --collapse"))?;
    tester
        .run(format!("{input} | table --collapse --no-headers"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
fn no_headers_preserves_sparse_rows(#[case] mode: &str) -> Result {
    let actual: String = test().run(format!(
        "[{{first_label: first_value}} {{second_label: last_value}}]
        | table {mode} --no-headers --index false --width 80"
    ))?;
    for label in ["first_label", "second_label"] {
        assert_contains_not(label, &actual);
    }
    for value in ["first_value", "last_value"] {
        assert_contains(value, &actual);
    }
    Ok(())
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
fn no_headers_removes_markdown_header_separator(#[case] mode: &str) -> Result {
    test()
        .run_with_data(
            format!("table {mode} --no-headers --index false --theme markdown"),
            test_table![["first_label", "second_label"]; [11, 22], [33, 44]],
        )
        .expect_value_eq("| 11 | 22 |\n| 33 | 44 |\n")
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_ignores_hidden_label_widths(#[case] mode: &str) -> Result {
    test()
        .run_with_data(
            format!("table {mode} --no-headers --index false --theme basic --width 11"),
            test_table![["very_long_first_label", "very_long_second_label"]; [11, 22]],
        )
        .expect_value_eq(indoc! {"
            +----+----+
            | 11 | 22 |
            +----+----+
        "})
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_applies_to_every_stream_page(#[case] mode: &str) -> Result {
    let actual: String = test().run(format!(
        "$env.config.table.stream_page_size = 2
        0..3 | each {{|i| {{column_label: $'item($i)'}} }}
        | table {mode} --no-headers --index 9 --width 80"
    ))?;
    assert_contains_not("column_label", &actual);
    assert_contains_not("#", &actual);
    for i in 0..4 {
        assert_eq!(actual.matches(&format!("item{i}")).count(), 1, "{actual}");
        assert_contains(format!(" {} ", i + 9), &actual);
    }
    Ok(())
}

#[rstest]
#[case::general("")]
#[case::expanded("--expand")]
#[case::collapsed("--collapse")]
fn no_headers_false_keeps_default_output(#[case] mode: &str) -> Result {
    let mut tester = test();
    let input = "[[first_label second_label]; [11 22] [33 44]]";
    let expected: String = tester.run(format!("{input} | table {mode}"))?;
    assert_contains("first_label", &expected);
    assert_contains("second_label", &expected);
    tester
        .run(format!("{input} | table {mode} --no-headers=false"))
        .expect_value_eq(expected)
}
