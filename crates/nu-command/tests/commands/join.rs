use nu_protocol::{ShellError, shell_error::ErrorSite};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn cases_where_result_is_same_between_join_types_inner() -> Result {
    do_cases_where_result_is_same_between_join_types("--inner")
}

#[test]
fn cases_where_result_differs_between_join_types_inner() -> Result {
    do_cases_where_result_differs_between_join_types("--inner")
}

#[test]
fn cases_where_result_differs_between_join_types_with_different_join_keys_inner() -> Result {
    do_cases_where_result_differs_between_join_types_with_different_join_keys("--inner")
}

#[test]
fn cases_where_result_is_same_between_join_types_left() -> Result {
    do_cases_where_result_is_same_between_join_types("--left")
}

#[test]
fn cases_where_result_is_same_between_join_types_outer() -> Result {
    do_cases_where_result_is_same_between_join_types("--outer")
}

#[test]
fn cases_where_result_differs_between_join_types_left() -> Result {
    do_cases_where_result_differs_between_join_types("--left")
}

#[test]
fn cases_where_result_differs_between_join_types_with_different_join_keys_left() -> Result {
    do_cases_where_result_differs_between_join_types_with_different_join_keys("--left")
}

#[test]
fn cases_where_result_is_same_between_join_types_right() -> Result {
    do_cases_where_result_is_same_between_join_types("--right")
}

#[test]
fn cases_where_result_differs_between_join_types_right() -> Result {
    do_cases_where_result_differs_between_join_types("--right")
}

#[test]
fn cases_where_result_differs_between_join_types_outer() -> Result {
    do_cases_where_result_differs_between_join_types("--outer")
}

#[test]
fn cases_where_result_differs_between_join_types_with_different_join_keys_outer() -> Result {
    do_cases_where_result_differs_between_join_types_with_different_join_keys("--outer")
}

#[test]
fn cases_where_result_differs_between_join_types_anti() -> Result {
    do_cases_where_result_differs_between_join_types("--anti")
}

#[test]
fn cases_where_result_differs_between_join_types_with_different_join_keys_anti() -> Result {
    do_cases_where_result_differs_between_join_types_with_different_join_keys("--anti")
}

fn do_cases_where_result_is_same_between_join_types(join_type: &str) -> Result {
    // .mode column
    // .headers on
    for ((left, right, on), expected) in [
        // Empty tables have no rows that could hold the join column, so any
        // column name is accepted for them.
        (("[]", "[]", "_"), "[]"),
        (("[{a: 1}]", "[{a: 1}]", "a"), "[[a]; [1]]"),
        (("[{a: 1} {a: 1}]", "[{a: 1}]", "a"), "[[a]; [1], [1]]"),
        (("[{a: 1}]", "[{a: 1} {a: 1}]", "a"), "[[a]; [1], [1]]"),
        (
            ("[{a: 1} {a: 1}]", "[{a: 1} {a: 1}]", "a"),
            "[[a]; [1], [1], [1], [1]]",
        ),
        (("[{a: 1 b: 1}]", "[{a: 1}]", "a"), "[[a, b]; [1, 1]]"),
        (("[{a: 1}]", "[{a: 1 b: 2}]", "a"), "[[a, b]; [1, 2]]"),
        (
            // create table l (a, b);
            // create table r (a, b);
            // insert into l (a, b) values (1, 1);
            // insert into r (a, b) values (1, 2);
            // select * from l inner join r on l.a = r.a;
            ("[{a: 1 b: 1}]", "[{a: 1 b: 2}]", "a"),
            "[[a, b, b_]; [1, 1, 2]]",
        ),
        (("[{a: 1}]", "[{a: 1 b: 1}]", "a"), "[[a, b]; [1, 1]]"),
    ] {
        let expr = format!("{left} | join {right} {join_type} {on} | to nuon");
        assert_join_output(&expr, expected)?;

        // Test again with streaming input (using `each` to convert the input into a ListStream)
        let to_list_stream = "each { |i| $i } | ";
        let expr = format!("{left} | {to_list_stream} join {right} {join_type} {on} | to nuon");
        assert_join_output(&expr, expected)?;
    }
    Ok(())
}

fn do_cases_where_result_differs_between_join_types(join_type: &str) -> Result {
    // .mode column
    // .headers on
    for ((left, right, on), join_types) in [
        (
            ("[]", "[{a: 1}]", "a"),
            [
                ("--inner", "[]"),
                ("--left", "[]"),
                ("--right", "[[a]; [1]]"),
                ("--outer", "[[a]; [1]]"),
                ("--anti", "[]"),
            ],
        ),
        (
            ("[{a: 1}]", "[]", "a"),
            [
                ("--inner", "[]"),
                ("--left", "[[a]; [1]]"),
                ("--right", "[]"),
                ("--outer", "[[a]; [1]]"),
                ("--anti", "[[a]; [1]]"),
            ],
        ),
        (
            ("[{a: 2 b: 1}]", "[{a: 1}]", "a"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, b]; [2, 1]]"),
                ("--right", "[[a, b]; [1, null]]"),
                ("--outer", "[[a, b]; [2, 1], [1, null]]"),
                ("--anti", "[[a, b]; [2, 1]]"),
            ],
        ),
        (
            ("[{a: 1}]", "[{a: 2 b: 1}]", "a"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, b]; [1, null]]"),
                ("--right", "[[a, b]; [2, 1]]"),
                ("--outer", "[[a, b]; [1, null], [2, 1]]"),
                ("--anti", "[[a]; [1]]"),
            ],
        ),
        (
            // create table l (a, b);
            // create table r (a, b);
            // insert into l (a, b) values (1, 2);
            // insert into r (a, b) values (2, 1);
            ("[{a: 1 b: 2}]", "[{a: 2 b: 1}]", "a"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, b, b_]; [1, 2, null]]"),
                // select * from l right outer join r on l.a = r.a;
                ("--right", "[[a, b, b_]; [2, null, 1]]"),
                ("--outer", "[[a, b, b_]; [1, 2, null], [2, null, 1]]"),
                ("--anti", "[[a, b]; [1, 2]]"),
            ],
        ),
        (
            ("[{a: 1 b: 2}]", "[{a: 2 b: 1} {a: 1 b: 1}]", "a"),
            [
                ("--inner", "[[a, b, b_]; [1, 2, 1]]"),
                ("--left", "[[a, b, b_]; [1, 2, 1]]"),
                ("--right", "[[a, b, b_]; [2, null, 1], [1, 2, 1]]"),
                ("--outer", "[[a, b, b_]; [1, 2, 1], [2, null, 1]]"),
                ("--anti", "[]"),
            ],
        ),
        (
            (
                "[{a: 1 b: 1} {a: 2 b: 2} {a: 3 b: 3}]",
                "[{a: 1 c: 1} {a: 3 c: 3}]",
                "a",
            ),
            [
                ("--inner", "[[a, b, c]; [1, 1, 1], [3, 3, 3]]"),
                ("--left", "[[a, b, c]; [1, 1, 1], [2, 2, null], [3, 3, 3]]"),
                ("--right", "[[a, b, c]; [1, 1, 1], [3, 3, 3]]"),
                ("--outer", "[[a, b, c]; [1, 1, 1], [2, 2, null], [3, 3, 3]]"),
                ("--anti", "[[a, b]; [2, 2]]"),
            ],
        ),
        (
            // create table l (a, c);
            // create table r (a, b);
            // insert into l (a, c) values (1, 1), (2, 2), (3, 3);
            // insert into r (a, b) values (1, 1), (3, 3), (4, 4);
            (
                "[{a: 1 c: 1} {a: 2 c: 2} {a: 3 c: 3}]",
                "[{a: 1 b: 1} {a: 3 b: 3} {a: 4 b: 4}]",
                "a",
            ),
            [
                ("--inner", "[[a, c, b]; [1, 1, 1], [3, 3, 3]]"),
                ("--left", "[[a, c, b]; [1, 1, 1], [2, 2, null], [3, 3, 3]]"),
                // select * from l right outer join r on l.a = r.a;
                ("--right", "[[a, c, b]; [1, 1, 1], [3, 3, 3], [4, null, 4]]"),
                (
                    "--outer",
                    "[[a, c, b]; [1, 1, 1], [2, 2, null], [3, 3, 3], [4, null, 4]]",
                ),
                ("--anti", "[[a, c]; [2, 2]]"),
            ],
        ),
        (
            // a row in the left table does not have the join column
            (
                "[{a: 1 ref: 1} {a: 2 ref: 2} {a: 3}]",
                "[{ref: 1 b: 1} {ref: 2 b: 2} {ref: 3 b: 3}]",
                "ref",
            ),
            [
                ("--inner", "[[a, ref, b]; [1, 1, 1], [2, 2, 2]]"),
                (
                    "--left",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [3, null, null]]",
                ),
                (
                    "--right",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [null, 3, 3]]",
                ),
                (
                    "--outer",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [3, null, null], [null, 3, 3]]",
                ),
                ("--anti", "[[a]; [3]]"),
            ],
        ),
        (
            // a row in the right table does not have the join column
            (
                "[{a: 1 ref: 1} {a: 2 ref: 2} {a: 3 ref: 3}]",
                "[{ref: 1 b: 1} {ref: 2 b: 2} {b: 3}]",
                "ref",
            ),
            [
                ("--inner", "[[a, ref, b]; [1, 1, 1], [2, 2, 2]]"),
                (
                    "--left",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [3, 3, null]]",
                ),
                (
                    "--right",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [null, null, 3]]",
                ),
                (
                    "--outer",
                    "[[a, ref, b]; [1, 1, 1], [2, 2, 2], [3, 3, null], [null, null, 3]]",
                ),
                ("--anti", "[[a, ref]; [3, 3]]"),
            ],
        ),
    ] {
        for (join_type_, expected) in join_types {
            if join_type_ == join_type {
                let expr = format!("{left} | join {right} {join_type} {on} | to nuon");
                assert_join_output(&expr, expected)?;

                // Test again with streaming input (using `each` to convert the input into a ListStream)
                let to_list_stream = "each { |i| $i } | ";
                let expr =
                    format!("{left} | {to_list_stream} join {right} {join_type} {on} | to nuon");
                assert_join_output(&expr, expected)?;
            }
        }
    }
    Ok(())
}

fn do_cases_where_result_differs_between_join_types_with_different_join_keys(
    join_type: &str,
) -> Result {
    // .mode column
    // .headers on
    for ((left, right, left_on, right_on), join_types) in [
        (
            ("[]", "[{z: 1}]", "a", "z"),
            [
                ("--inner", "[]"),
                ("--left", "[]"),
                ("--right", "[[z]; [1]]"),
                ("--outer", "[[z]; [1]]"),
                ("--anti", "[]"),
            ],
        ),
        (
            ("[{a: 1}]", "[]", "a", "z"),
            [
                ("--inner", "[]"),
                ("--left", "[[a]; [1]]"),
                ("--right", "[]"),
                ("--outer", "[[a]; [1]]"),
                ("--anti", "[[a]; [1]]"),
            ],
        ),
        (
            ("[{a: 2 b: 1}]", "[{z: 1}]", "a", "z"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, b, z]; [2, 1, null]]"),
                ("--right", "[[a, b, z]; [null, null, 1]]"),
                ("--outer", "[[a, b, z]; [2, 1, null], [null, null, 1]]"),
                ("--anti", "[[a, b]; [2, 1]]"),
            ],
        ),
        (
            ("[{a: 1}]", "[{z: 2 b: 1}]", "a", "z"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, z, b]; [1, null, null]]"),
                ("--right", "[[a, z, b]; [null, 2, 1]]"),
                ("--outer", "[[a, z, b]; [1, null, null], [null, 2, 1]]"),
                ("--anti", "[[a]; [1]]"),
            ],
        ),
        (
            // create table l (a, b);
            // create table r (a, b);
            // insert into l (a, b) values (1, 2);
            // insert into r (a, b) values (2, 1);
            ("[{a: 1 b: 2}]", "[{z: 2 b: 1}]", "a", "z"),
            [
                ("--inner", "[]"),
                ("--left", "[[a, b, z, b_]; [1, 2, null, null]]"),
                // select * from l right outer join r on l.a = r.z;
                ("--right", "[[a, b, z, b_]; [null, null, 2, 1]]"),
                (
                    "--outer",
                    "[[a, b, z, b_]; [1, 2, null, null], [null, null, 2, 1]]",
                ),
                ("--anti", "[[a, b]; [1, 2]]"),
            ],
        ),
        (
            ("[{a: 1 b: 2}]", "[{z: 2 b: 1} {z: 1 b: 1}]", "a", "z"),
            [
                ("--inner", "[[a, b, z, b_]; [1, 2, 1, 1]]"),
                ("--left", "[[a, b, z, b_]; [1, 2, 1, 1]]"),
                (
                    "--right",
                    "[[a, b, z, b_]; [null, null, 2, 1], [1, 2, 1, 1]]",
                ),
                (
                    "--outer",
                    "[[a, b, z, b_]; [1, 2, 1, 1], [null, null, 2, 1]]",
                ),
                ("--anti", "[]"),
            ],
        ),
        (
            (
                "[{a: 1 b: 1} {a: 2 b: 2} {a: 3 b: 3}]",
                "[{z: 1 c: 1} {z: 3 c: 3}]",
                "a",
                "z",
            ),
            [
                ("--inner", "[[a, b, z, c]; [1, 1, 1, 1], [3, 3, 3, 3]]"),
                (
                    "--left",
                    "[[a, b, z, c]; [1, 1, 1, 1], [2, 2, null, null], [3, 3, 3, 3]]",
                ),
                ("--right", "[[a, b, z, c]; [1, 1, 1, 1], [3, 3, 3, 3]]"),
                (
                    "--outer",
                    "[[a, b, z, c]; [1, 1, 1, 1], [2, 2, null, null], [3, 3, 3, 3]]",
                ),
                ("--anti", "[[a, b]; [2, 2]]"),
            ],
        ),
        (
            // create table l (a, c);
            // create table r (a, b);
            // insert into l (a, c) values (1, 1), (2, 2), (3, 3);
            // insert into r (a, b) values (1, 1), (3, 3), (4, 4);
            (
                "[{a: 1 c: 1} {a: 2 c: 2} {a: 3 c: 3}]",
                "[{z: 1 b: 1} {z: 3 b: 3} {z: 4 b: 4}]",
                "a",
                "z",
            ),
            [
                ("--inner", "[[a, c, z, b]; [1, 1, 1, 1], [3, 3, 3, 3]]"),
                (
                    "--left",
                    "[[a, c, z, b]; [1, 1, 1, 1], [2, 2, null, null], [3, 3, 3, 3]]",
                ),
                // select * from l right outer join r on l.a = r.z;
                (
                    "--right",
                    "[[a, c, z, b]; [1, 1, 1, 1], [3, 3, 3, 3], [null, null, 4, 4]]",
                ),
                (
                    "--outer",
                    "[[a, c, z, b]; [1, 1, 1, 1], [2, 2, null, null], [3, 3, 3, 3], [null, null, 4, 4]]",
                ),
                ("--anti", "[[a, c]; [2, 2]]"),
            ],
        ),
    ] {
        for (join_type_, expected) in join_types {
            if join_type_ == join_type {
                let expr =
                    format!("{left} | join {right} {join_type} {left_on} {right_on} | to nuon");
                assert_join_output(&expr, expected)?;

                // Test again with streaming input (using `each` to convert the input into a ListStream)
                let to_list_stream = "each { |i| $i } | ";
                let expr = format!(
                    "{left} | {to_list_stream} join {right} {join_type} {left_on} {right_on} | to nuon"
                );
                assert_join_output(&expr, expected)?;
            }
        }
    }
    Ok(())
}

#[test]
fn test_anti_join() -> Result {
    for ((left, right, on), expected) in [
        (
            (
                "[[a, ref]; [1, 2] [1, 2] [5, 3]]",
                "[[ref]; [3] [4] [5]]",
                "ref",
            ),
            "[[a, ref]; [1, 2], [1, 2]]",
        ),
        (
            ("[[a, ref]; [5, 3]]", "[[ref]; [3], [3], [4]]", "ref"),
            "[]",
        ),
    ] {
        let expr = format!("{left} | join {right} --anti {on} | to nuon");

        assert_join_output(&expr, expected)?;

        // Test again with streaming input (using `each` to convert the input into a ListStream)
        // let to_list_stream = "each { |i| $i } | ";
        // let expr = format!("{left} | {to_list_stream} join {right} --anti {on} | to nuon");
        // assert_join_output(&expr, expected)?;
    }
    Ok(())
}

#[test]
fn test_alternative_table_syntax() -> Result {
    let join_type = "--inner";
    for ((left, right, on), expected) in [
        (("[{a: 1}]", "[{a: 1}]", "a"), "[[a]; [1]]"),
        (("[{a: 1}]", "[[a]; [1]]", "a"), "[[a]; [1]]"),
        (("[[a]; [1]]", "[{a: 1}]", "a"), "[[a]; [1]]"),
        (("[[a]; [1]]", "[[a]; [1]]", "a"), "[[a]; [1]]"),
    ] {
        let expr = format!("{left} | join {right} {join_type} {on} | to nuon");
        assert_join_output(&expr, expected)?;
    }
    Ok(())
}

// A non-empty table where no row has the join column is an error, for every
// join type and for streamed input. `right_and_keys` is the `join` arguments
// after the flag: the right table, `left-on` and the optional `right-on`. The
// error names `table` and points at the `missing` column argument, so the
// checks of the two tables cannot swap their key or table without failing.
#[rstest]
#[case::neither_table_has_key("[[a b]; [1 2] [3 4]]", "[[a c]; [1 3] [4 5]] d", "input", "d")]
#[case::left_table_lacks_key("[{b: 1}]", "[{a: 1}] a", "input", "a")]
#[case::right_table_lacks_key("[{a: 1}]", "[{b: 1}] a", "right", "a")]
#[case::left_table_lacks_left_on("[{a: 1}]", "[{z: 1}] b z", "input", "b")]
#[case::right_table_lacks_right_on("[{a: 1}]", "[{z: 1}] a y", "right", "y")]
#[case::left_empty_right_lacks_key("[]", "[{b: 1}] a", "right", "a")]
#[case::right_empty_left_lacks_key("[{b: 1}]", "[] a", "input", "a")]
fn join_errors_when_table_has_no_join_column(
    #[case] left: &str,
    #[case] right_and_keys: &str,
    #[case] table: &str,
    #[case] missing: &str,
    #[values("--inner", "--left", "--right", "--outer", "--anti")] join_type: &str,
    #[values("", "each { |i| $i } | ")] to_list_stream: &str,
) -> Result {
    let code = format!("{left} | {to_list_stream}join {join_type} {right_and_keys}");
    let mut tester = test();
    let err = match tester.run(code).expect_shell_error()? {
        ShellError::Generic(err) => err,
        err => panic!("expected a generic error, got {err:?}"),
    };
    assert_eq!(err.code, "nu::shell::column_not_found");
    assert_eq!(err.error, format!("Cannot find column '{missing}'"));
    assert_eq!(
        err.msg,
        format!("no row in the {table} table has this column")
    );
    let ErrorSite::Span(span) = err.site else {
        panic!("expected the error to point at the column argument");
    };
    let span_text = String::from_utf8_lossy(tester.engine_state.get_span_contents(span));
    assert_eq!(span_text, missing);
    Ok(())
}

#[test]
fn test_incompatible_anti_join_args() -> Result {
    let err = test()
        .run("[] | join --anti --prefix r_ [] ref")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, .. }
        if msg == "--prefix is a no-op under --anti"
    );

    let err = test()
        .run("[] | join --anti --suffix r_ [] ref")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, .. }
        if msg == "--suffix is a no-op under --anti"
    );

    Ok(())
}

fn assert_join_output(expr: &str, expected: &str) -> Result {
    test().run(expr).expect_value_eq(expected)
}
