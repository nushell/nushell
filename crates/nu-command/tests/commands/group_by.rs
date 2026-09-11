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
    // Int keys are not strings, so the default output is a table.
    test()
        .run("[{foo: 123}, {foo: 234}, {bar: 345}] | group-by foo?")
        .expect_value_eq(test_value!([
            {foo: 123, items: [{foo: 123}]},
            {foo: 234, items: [{foo: 234}]},
        ]))
}

#[test]
fn group_by_compound_values_are_grouped_distinctly() -> Result {
    // List keys force table output. Distinct lists stay distinct.
    test()
        .run("[[k v]; [a [2 1]] [b [1 2]] [c [3]] [d [2]]] | group-by v")
        .expect_value_eq(test_value!([
            {v: [2, 1], items: [{k: "a", v: [2, 1]}]},
            {v: [1, 2], items: [{k: "b", v: [1, 2]}]},
            {v: [3], items: [{k: "c", v: [3]}]},
            {v: [2], items: [{k: "d", v: [2]}]},
        ]))
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
        .expect_value_eq(test_value!([
            {foo: 123, items: [{foo: 123}]},
            {foo: 234, items: [{foo: 234}]},
        ]))?;

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
    // --to-table keeps original key types; null is preserved as nothing.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b --to-table")
        .expect_value_eq(test_value!([
            {a: (), b: 1, items: [{a: (), b: 1}]},
            {a: 2, b: 1, items: [{a: 2, b: 1}]},
        ]))?;

    // Non-string keys (and null) force table output; null is kept as nothing.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b")
        .expect_value_eq(test_value!([
            {a: (), b: 1, items: [{a: (), b: 1}]},
            {a: 2, b: 1, items: [{a: 2, b: 1}]},
        ]))
}

#[test]
fn to_table_preserves_filesize_keys_and_does_not_collapse_display_collisions() -> Result {
    test()
        .run("[[size]; [1MB] [1.001MB]] | group-by size --to-table | length")
        .expect_value_eq(2)?;

    let code = "
        let data = [[size]; [1MB] [1.001MB]]
        let grouped = $data | group-by size --to-table
        $grouped.size == $data.size
    ";
    test().run(code).expect_value_eq(true)?;

    test()
        .run("[[size]; [1MB]] | group-by size --to-table | get 0.size | describe")
        .expect_value_eq("filesize")?;

    test()
        .run("[[size]; [1MB] [1.001MB]] | group-by size --to-table | get 0.size | into filesize")
        .expect_value_eq(nu_protocol::Filesize::new(1_000_000))
}

#[test]
fn to_table_groups_equal_lists_and_keeps_list_keys() -> Result {
    test()
        .run("[[k v]; [a [1]] [b [1]]] | group-by v --to-table")
        .expect_value_eq(test_value!([
            {v: [1], items: [{k: "a", v: [1]}, {k: "b", v: [1]}]},
        ]))
}

#[test]
fn to_table_keeps_int_keys() -> Result {
    test()
        .run("[{n: 1} {n: 1} {n: 2}] | group-by n --to-table")
        .expect_value_eq(test_value!([
            {n: 1, items: [{n: 1}, {n: 1}]},
            {n: 2, items: [{n: 2}]},
        ]))
}

#[test]
fn non_string_keys_emit_a_table_without_to_table_flag() -> Result {
    test()
        .run("[[size]; [1MB] [1.001MB]] | group-by size | length")
        .expect_value_eq(2)?;

    test()
        .run("[[size]; [1MB]] | group-by size | get 0.size | describe")
        .expect_value_eq("filesize")?;

    test()
        .run("[{n: 1} {n: 1} {n: 2}] | group-by n")
        .expect_value_eq(test_value!([
            {n: 1, items: [{n: 1}, {n: 1}]},
            {n: 2, items: [{n: 2}]},
        ]))?;

    test()
        .run("[1 2 1] | group-by")
        .expect_value_eq(test_value!([
            {group: 1, items: [1, 1]},
            {group: 2, items: [2]},
        ]))?;

    test()
        .run(r#"[true "true"] | group-by"#)
        .expect_value_eq(test_value!([
            {group: true, items: [true]},
            {group: "true", items: ["true"]},
        ]))?;

    test()
        .run(r#"["a" 1] | group-by"#)
        .expect_value_eq(test_value!([
            {group: "a", items: ["a"]},
            {group: 1, items: [1]},
        ]))
}

#[test]
fn string_keys_still_emit_a_record() -> Result {
    test()
        .run("['a' 'b' 'a'] | group-by")
        .expect_value_eq(test_value!({
            a: ["a", "a"],
            b: ["b"],
        }))
}

#[test]
fn items_grouper_errors_when_output_is_a_table() -> Result {
    // String keys still emit a record, so a column named `items` is fine.
    test()
        .run(r#"[{items: "a"} {items: "a"}] | group-by items"#)
        .expect_value_eq(test_value!({
            a: [{items: "a"}, {items: "a"}],
        }))?;

    // Non-string keys emit a table, which cannot have two `items` columns.
    let err = test()
        .run("[{items: 1} {items: 2}] | group-by items")
        .expect_shell_error()?;
    match err {
        ShellError::Generic(generic) => {
            assert_contains("items", generic.error.as_ref());
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn closures_with_different_captures_are_distinct_groups() -> Result {
    let code = "
        let make = {|n| {|| $n }}
        [(do $make 1) (do $make 1) (do $make 2)] | group-by --to-table | length
    ";
    test().run(code).expect_value_eq(2)
}

#[test]
fn to_table_does_not_merge_int_and_float_ranges() -> Result {
    // `1..3 == 1.0..3.0` is true, but --to-table groups by typed identity.
    test()
        .run("[1..3, 1..3, 1.0..3.0] | group-by --to-table | length")
        .expect_value_eq(2)?;

    test()
        .run("[1..3, 1..3] | group-by --to-table | length")
        .expect_value_eq(1)?;

    test()
        .run("[1.0..3.0, 1.0..3.0] | group-by --to-table | length")
        .expect_value_eq(1)?;

    test()
        .run("[1..3, 1..4] | group-by --to-table | length")
        .expect_value_eq(2)?;

    test()
        .run("[1..3] | group-by --to-table | get 0.group | describe")
        .expect_value_eq("range")?;

    let code = "
        let grouped = [1..3, 1..3, 1.0..3.0] | group-by --to-table
        ($grouped.items.0 | length) == 2 and ($grouped.items.1 | length) == 1
    ";
    test().run(code).expect_value_eq(true)
}
