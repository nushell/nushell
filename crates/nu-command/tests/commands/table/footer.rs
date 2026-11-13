use indoc::indoc;
use nu_test_support::prelude::*;

#[test]
fn table_footer_inheritance() -> Result {
    let field0 = test_table![
        ["y1", "y2", "y3"];
        [1, 2, 3],
        [79, 79, 79],
        [
            test_value!({
                f1: "a string",
                f2: 1000,
            }),
            1,
            2,
        ],
    ];
    let field3 = Value::test_list(
        (0..212)
            .map(|_| {
                test_record! {
                    "head1" => 79,
                    "head2" => 79,
                    "head3" => 79,
                }
            })
            .collect(),
    );
    let field5 = test_table![
        ["x1", "x2", "x3"];
        [1, 2, 3],
        [79, 79, 79],
        [
            test_value!({
                f1: "a string",
                f2: 1000,
            }),
            1,
            2,
        ],
    ];
    let actual: String = test().run_with_data(
        "
            let table = $in
            $env.config.table.footer_inheritance = true
            $table
            | table --width=80 --expand
        ",
        test_value!({
            field0: (field0),
            field1: ["a", "b", "c"],
            field2: [123, 234, 345],
            field3: (field3),
            field4: {
                f1: 1,
                f2: 3,
                f3: {
                    f1: "f1",
                    f2: "f2",
                    f3: "f3",
                },
            },
            field5: (field5),
        }),
    )?;
    assert_eq!(actual.match_indices("head1").count(), 2);
    assert_eq!(actual.match_indices("head2").count(), 2);
    assert_eq!(actual.match_indices("head3").count(), 2);
    assert_eq!(actual.match_indices("y1").count(), 1);
    assert_eq!(actual.match_indices("y2").count(), 1);
    assert_eq!(actual.match_indices("y3").count(), 1);
    assert_eq!(actual.match_indices("x1").count(), 1);
    assert_eq!(actual.match_indices("x2").count(), 1);
    assert_eq!(actual.match_indices("x3").count(), 1);
    Ok(())
}

#[test]
fn table_footer_inheritance_kv_rows() -> Result {
    let mut tester = test();
    let code = "
        let data = $in
        $env.config.table.footer_inheritance = true
        $env.config.footer_mode = 7
        $data
        | table --expand --width=80
    ";
    tester
        .run_with_data(
            code,
            test_value!([
                {
                    a: "kv",
                    b: {
                        "0": 0,
                        "1": 1,
                        "2": 2,
                        "3": 3,
                        "4": 4,
                    },
                },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────╮
            │ # │  a   │     b     │
            ├───┼──────┼───────────┤
            │ 0 │ kv   │ ╭───┬───╮ │
            │   │      │ │ 0 │ 0 │ │
            │   │      │ │ 1 │ 1 │ │
            │   │      │ │ 2 │ 2 │ │
            │   │      │ │ 3 │ 3 │ │
            │   │      │ │ 4 │ 4 │ │
            │   │      │ ╰───┴───╯ │
            │ 1 │ data │         0 │
            │ 2 │ data │         0 │
            ╰───┴──────┴───────────╯
        "})?;
    tester
        .run_with_data(
            code,
            test_value!([
                {
                    a: "kv",
                    b: {
                        "0": 0,
                        "1": 1,
                        "2": 2,
                        "3": 3,
                        "4": 4,
                        "5": 5,
                    },
                },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────╮
            │ # │  a   │     b     │
            ├───┼──────┼───────────┤
            │ 0 │ kv   │ ╭───┬───╮ │
            │   │      │ │ 0 │ 0 │ │
            │   │      │ │ 1 │ 1 │ │
            │   │      │ │ 2 │ 2 │ │
            │   │      │ │ 3 │ 3 │ │
            │   │      │ │ 4 │ 4 │ │
            │   │      │ │ 5 │ 5 │ │
            │   │      │ ╰───┴───╯ │
            │ 1 │ data │         0 │
            │ 2 │ data │         0 │
            ├───┼──────┼───────────┤
            │ # │  a   │     b     │
            ╰───┴──────┴───────────╯
        "})
}

#[test]
fn table_footer_inheritance_list_rows() -> Result {
    let mut tester = test();
    let code = "
        let data = $in
        $env.config.table.footer_inheritance = true
        $env.config.footer_mode = 7
        $data
        | table --expand --width=80
    ";
    tester
        .run_with_data(
            code,
            test_value!([
                {
                    a: "kv",
                    b: {
                        "0": (test_table![
                            ["field"];
                            [0],
                            [1],
                            [2],
                            [3],
                            [4],
                        ]),
                    },
                },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────────────────╮
            │ # │  a   │           b           │
            ├───┼──────┼───────────────────────┤
            │ 0 │ kv   │ ╭───┬───────────────╮ │
            │   │      │ │   │ ╭───┬───────╮ │ │
            │   │      │ │ 0 │ │ # │ field │ │ │
            │   │      │ │   │ ├───┼───────┤ │ │
            │   │      │ │   │ │ 0 │     0 │ │ │
            │   │      │ │   │ │ 1 │     1 │ │ │
            │   │      │ │   │ │ 2 │     2 │ │ │
            │   │      │ │   │ │ 3 │     3 │ │ │
            │   │      │ │   │ │ 4 │     4 │ │ │
            │   │      │ │   │ ╰───┴───────╯ │ │
            │   │      │ ╰───┴───────────────╯ │
            │ 1 │ data │                     0 │
            │ 2 │ data │                     0 │
            ╰───┴──────┴───────────────────────╯
        "})?;
    tester
        .run_with_data(
            code,
            test_value!([
                {
                    a: "kv",
                    b: {
                        "0": (test_table![
                            ["field"];
                            [0],
                            [1],
                            [2],
                            [3],
                            [4],
                            [5],
                        ]),
                    },
                },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────────────────╮
            │ # │  a   │           b           │
            ├───┼──────┼───────────────────────┤
            │ 0 │ kv   │ ╭───┬───────────────╮ │
            │   │      │ │   │ ╭───┬───────╮ │ │
            │   │      │ │ 0 │ │ # │ field │ │ │
            │   │      │ │   │ ├───┼───────┤ │ │
            │   │      │ │   │ │ 0 │     0 │ │ │
            │   │      │ │   │ │ 1 │     1 │ │ │
            │   │      │ │   │ │ 2 │     2 │ │ │
            │   │      │ │   │ │ 3 │     3 │ │ │
            │   │      │ │   │ │ 4 │     4 │ │ │
            │   │      │ │   │ │ 5 │     5 │ │ │
            │   │      │ │   │ ╰───┴───────╯ │ │
            │   │      │ ╰───┴───────────────╯ │
            │ 1 │ data │                     0 │
            │ 2 │ data │                     0 │
            ├───┼──────┼───────────────────────┤
            │ # │  a   │           b           │
            ╰───┴──────┴───────────────────────╯
        "})
}
