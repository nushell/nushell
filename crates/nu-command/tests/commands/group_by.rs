use nu_protocol::test_value;
use nu_test_support::prelude::*;

#[test]
fn groups() -> Result {
    let code = r#"
        [
            [first_name, last_name, rusty_at, type];
            [Andrés, Robalino, "10/11/2013", A],
            [JT, Turner, "10/12/2013", B],
            [Yehuda, Katz, "10/11/2013", A]
        ]
        | group-by rusty_at
        | get "10/11/2013"
        | length
    "#;

    test().run(code).expect_value_eq(2)
}

#[test]
fn errors_if_given_unknown_column_name() -> Result {
    let code = r#"
        [{
            nu: {
                committers: [
                    {name: "Andrés N. Robalino"},
                    {name: "JT Turner"},
                    {name: "Yehuda Katz"}
                ],
                releases: [
                    {version: "0.2"},
                    {version: "0.8"},
                    {version: "0.9999999"}
                ],
                "0xATYKARNU": [
                    ["Th", "e", " "],
                    ["BIG", " ", "UnO"],
                    ["punto", "cero"]
                ]
            }
        }]
        | group-by { get nu.releases.missing_column }
    "#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "missing_column");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn errors_if_column_not_found() -> Result {
    let code = r#"
        [
            [first_name, last_name, rusty_at, type];
            [Andrés, Robalino, "10/11/2013", A],
            [JT, Turner, "10/12/2013", B],
            [Yehuda, Katz, "10/11/2013", A]
        ]
        | group-by ttype
    "#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::DidYouMean { suggestion, .. } => {
            assert_eq!(suggestion, "type");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn group_by_on_empty_list_returns_empty_record() -> Result {
    test()
        .run("[[a b]; [1 2]] | where false | group-by a")
        .expect_value_eq(test_value!({}))
}

#[test]
fn group_by_to_table_on_empty_list_returns_empty_list() -> Result {
    test()
        .run("[[a b]; [1 2]] | where false | group-by --to-table a")
        .expect_value_eq(test_value!([]))
}

#[test]
fn optional_cell_path_works() -> Result {
    test()
        .run("[{foo: 123}, {foo: 234}, {bar: 345}] | group-by foo?")
        .expect_value_eq(test_value!({
            "123": [{foo: 123}],
            "234": [{foo: 234}],
        }))
}

#[test]
fn group_by_compound_values_are_grouped_distinctly() -> Result {
    // Regression test for grouping by list values.
    let code = "
        let data = [[k v]; [a [2 1]] [b [1 2]] [c [3]] [d [2]]]
        $data | group-by v | columns | length
    ";
    test().run(code).expect_value_eq(4)?;

    // Every distinct list value should produce a separate group with exactly 1 row.
    let code = "
        let data = [[k v]; [a [2 1]] [b [1 2]] [c [3]] [d [2]]]
        $data
        | group-by v
        | values
        | each {|items| $items | length }
        | uniq
        | first
    ";
    test().run(code).expect_value_eq(1)
}

// --- null key consistency (#18707) ---

#[test]
fn null_keys_omitted_from_record_for_list_cell_path_and_closure() -> Result {
    // List values: null is not mapped to "" and is omitted from record output.
    test()
        .run("[ a null ] | group-by | columns")
        .expect_value_eq(["a"])?;

    // Required cell path: explicit null column value is omitted from records.
    test()
        .run("[ { x: a } { x: null } ] | group-by x | columns")
        .expect_value_eq(["a"])?;

    // Closure: same policy as list/cell path.
    test()
        .run("[ { x: a } { x: null } ] | group-by { get x } | columns")
        .expect_value_eq(["a"])
}

#[test]
fn null_keys_included_in_to_table() -> Result {
    test()
        .run("[ a null ] | group-by --to-table")
        .expect_value_eq(test_value!([
            {group: "a", items: ["a"]},
            {group: (), items: [()]},
        ]))?;

    test()
        .run("[ { x: a } { x: null } ] | group-by x --to-table")
        .expect_value_eq(test_value!([
            {x: "a", items: [{x: "a"}]},
            {x: (), items: [{x: ()}]},
        ]))?;

    test()
        .run("[ { x: a } { x: null } ] | group-by { get x } --to-table")
        .expect_value_eq(test_value!([
            {closure_0: "a", items: [{x: "a"}]},
            {closure_0: (), items: [{x: ()}]},
        ]))
}

#[test]
fn null_and_empty_string_are_distinct_groups() -> Result {
    // Record: null omitted, empty string kept under "".
    test()
        .run(r#"[ a "" null ] | group-by"#)
        .expect_value_eq(test_value!({
            a: ["a"],
            "": [""],
        }))?;

    // Table: two separate group rows for "" and null.
    test()
        .run(r#"[ "" null ] | group-by --to-table"#)
        .expect_value_eq(test_value!([
            {group: "", items: [""]},
            {group: (), items: [()]},
        ]))?;

    test()
        .run(r#"[ { x: "" } { x: null } ] | group-by x --to-table | get x"#)
        .expect_value_eq(test_value!(["", ()]))
}

#[test]
fn optional_cell_path_still_skips_nothing() -> Result {
    // Missing optional column is still ignored (historical #9020 behavior).
    test()
        .run("[{foo: 123}, {foo: 234}, {bar: 345}] | group-by foo?")
        .expect_value_eq(test_value!({
            "123": [{foo: 123}],
            "234": [{foo: 234}],
        }))?;

    // Optional path with explicit null is also skipped (cannot distinguish from missing).
    test()
        .run("[{x: a}, {x: null}, {y: b}] | group-by x? | columns")
        .expect_value_eq(["a"])
}

#[test]
fn all_nulls_record_is_empty_table_has_null_group() -> Result {
    test()
        .run("[ null null ] | group-by")
        .expect_value_eq(test_value!({}))?;

    test()
        .run("[ null null ] | group-by --to-table")
        .expect_value_eq(test_value!([{group: (), items: [(), ()]}]))
}

#[test]
fn multi_grouper_null_key_in_to_table() -> Result {
    // Non-null keys are still stringified for grouping; null is preserved as nothing.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b --to-table")
        .expect_value_eq(test_value!([
            {a: (), b: "1", items: [{a: (), b: 1}]},
            {a: "2", b: "1", items: [{a: 2, b: 1}]},
        ]))?;

    // Record mode drops the null branch entirely.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b")
        .expect_value_eq(test_value!({
            "2": {
                "1": [{a: 2, b: 1}],
            },
        }))
}
