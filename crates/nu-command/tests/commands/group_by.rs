use nu_protocol::test_value;
use nu_test_support::prelude::*;
use rstest::rstest;

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
    // Missing optional column is still ignored (historical #9020 behavior). The keys are ints,
    // so the default output is a table.
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

    // `expect_value_eq` compares with `==`, where `1 == 1.0`, so check the key type directly.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b --to-table | get b | each { describe }")
        .expect_value_eq(["int", "int"])?;

    // Int keys force table output; the null group is kept as nothing.
    test()
        .run("[ { a: null, b: 1 } { a: 2, b: 1 } ] | group-by a b")
        .expect_value_eq(test_value!([
            {a: (), b: 1, items: [{a: (), b: 1}]},
            {a: 2, b: 1, items: [{a: 2, b: 1}]},
        ]))
}

#[test]
fn multi_grouper_record_omits_null_branch() -> Result {
    // String and null keys give a record, which omits the null branch.
    test()
        .run(r#"[ { a: null, b: "1" } { a: "2", b: "1" } ] | group-by a b"#)
        .expect_value_eq(test_value!({
            "2": {
                "1": [{a: "2", b: "1"}],
            },
        }))
}

#[test]
fn nested_non_string_keys_emit_a_table() -> Result {
    // A non-string key below the first grouper also needs a table.
    test()
        .run(r#"[ { a: "x", b: 1 } { a: "x", b: 2 } ] | group-by a b"#)
        .expect_value_eq(test_value!([
            {a: "x", b: 1, items: [{a: "x", b: 1}]},
            {a: "x", b: 2, items: [{a: "x", b: 2}]},
        ]))
}

#[rstest]
#[case::default("let data = [[size]; [1MB] [1.001MB]]; ($data | group-by size).size == $data.size")]
#[case::to_table(
    "let data = [[size]; [1MB] [1.001MB]]; ($data | group-by size --to-table).size == $data.size"
)]
fn filesize_keys_keep_their_type_and_do_not_collapse_display_collisions(
    #[case] code: &str,
) -> Result {
    // 1MB and 1.001MB both display as "1.0 MB". A filesize only equals another filesize, so the
    // comparison fails if the two groups merge or the keys turn into strings or numbers.
    test().run(code).expect_value_eq(true)
}

#[test]
fn to_table_groups_equal_lists_and_keeps_list_keys() -> Result {
    test()
        .run("[[k v]; [a [1]] [b [1]]] | group-by v --to-table")
        .expect_value_eq(test_value!([
            {v: [1], items: [{k: "a", v: [1]}, {k: "b", v: [1]}]},
        ]))
}

#[rstest]
#[case::default("[{n: 1} {n: 1} {n: 2}] | group-by n | update n { describe }")]
#[case::to_table("[{n: 1} {n: 1} {n: 2}] | group-by n --to-table | update n { describe }")]
fn int_keys_keep_their_type(#[case] code: &str) -> Result {
    // `expect_value_eq` compares with `==`, where `1 == 1.0`, so `describe` checks the key type.
    test().run(code).expect_value_eq(test_value!([
        {n: "int", items: [{n: 1}, {n: 1}]},
        {n: "int", items: [{n: 2}]},
    ]))
}

#[test]
fn non_string_keys_emit_a_table_without_to_table_flag() -> Result {
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

#[rstest]
#[case::items("[] | group-by items --to-table", "can't be named `items`")]
#[case::duplicate("[] | group-by a a --to-table", "colliding column names")]
fn to_table_checks_column_names_on_empty_input(
    #[case] code: &str,
    #[case] message: &str,
) -> Result {
    // `--to-table` always returns a table, so its column names are checked even with no rows.
    match test().run(code).expect_shell_error()? {
        ShellError::Generic(generic) => {
            assert_contains(message, generic.error.as_ref());
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn errors_in_the_input_are_raised() -> Result {
    test()
        .run("1..3 | each {|n| error make { msg: 'boom' } } | group-by")
        .expect_error_code_eq("nu::shell::eval_block_with_input")
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
    // `1..3 == 1.0..3.0` is true, but group keys keep their type. `describe` prints `range` for
    // both kinds, so the item counts show that int and float ranges stay apart.
    let code = "
        [1..3, 1..3, 1.0..3.0, 1.0..3.0, 1..4]
        | group-by --to-table
        | update group { describe }
        | update items { length }
    ";
    test().run(code).expect_value_eq(test_value!([
        {group: "range", items: 2},
        {group: "range", items: 2},
        {group: "range", items: 1},
    ]))
}

#[test]
fn int_and_float_keys_are_distinct_groups() -> Result {
    // `1 == 1.0` is true, but group keys keep their type.
    test()
        .run(
            "[1 1.0 1] | group-by --to-table | update group { describe } | update items { length }",
        )
        .expect_value_eq(test_value!([
            {group: "int", items: 2},
            {group: "float", items: 1},
        ]))
}

#[test]
fn equal_values_written_differently_share_a_group() -> Result {
    // Records ignore key order.
    test()
        .run("[{a: 1, b: 2} {b: 2, a: 1}] | group-by --to-table | length")
        .expect_value_eq(1)?;

    // Dates compare by instant, not by offset.
    test()
        .run("[2026-01-01T00:00:00+00:00 2026-01-01T02:00:00+02:00] | group-by --to-table | length")
        .expect_value_eq(1)?;

    // Floats compare with `==`, so `0.0` and `-0.0` are one key.
    test()
        .run("[0.0 -0.0] | group-by --to-table | length")
        .expect_value_eq(1)?;

    // NaN is one key although `NaN == NaN` is false. Keying by `==` instead of `strict_eq` would
    // give each row its own group.
    test()
        .run("[NaN NaN] | group-by --to-table | length")
        .expect_value_eq(1)
}

#[test]
fn custom_value_keys_group_by_their_own_equality() -> Result {
    test()
        .run(r#"["1.2.3" "1.3.0" "1.2.3"] | each { into semver } | group-by --to-table | get items | each { length }"#)
        .expect_value_eq(test_value!([2, 1]))?;

    test()
        .run(r#""1.2.3" | into semver | [$in] | group-by --to-table | get 0.group | describe"#)
        .expect_value_eq("semver")
}
