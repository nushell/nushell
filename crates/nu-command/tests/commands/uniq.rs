use Kind::*;
use nu_test_support::prelude::*;
use rstest::rstest;

#[derive(Debug, IntoValue, Clone)]
#[nu_value(rename_all = "SHOUTY_SNAKE_CASE")]
enum Kind {
    A,
    B,
    C,
}

#[derive(Debug, IntoValue, Clone)]
struct Sample {
    first_name: &'static str,
    last_name: &'static str,
    kind: Kind,
}

#[rustfmt::skip]
static SAMPLE: [Sample; 5] = [
    Sample { first_name: "Andrés", last_name: "Robalino", kind: A },
    Sample { first_name: "JT",     last_name: "Turner",   kind: B },
    Sample { first_name: "Yehuda", last_name: "Katz",     kind: A },
    Sample { first_name: "JT",     last_name: "Turner",   kind: B },
    Sample { first_name: "Yehuda", last_name: "Katz",     kind: A },
];

#[test]
fn removes_duplicate_rows() -> Result {
    test()
        .run_with_data("$in | uniq | length", SAMPLE.clone())
        .expect_value_eq(3)
}

#[test]
fn uniq_values() -> Result {
    test()
        .run_with_data("$in | select kind | uniq | length", SAMPLE.clone())
        .expect_value_eq(2)
}

#[test]
fn uniq_empty() -> Result {
    test().run("[] | uniq | to nuon").expect_value_eq("[]")
}

#[test]
fn nested_json_structures() -> Result {
    let sample = r#"
        [
            {
                "name": "this is duplicated",
                "nesting": [{ "a": "a", "b": "b" }, { "c": "c", "d": "d" }],
                "can_be_ordered_differently": {
                    "array": [1, 2, 3, 4, 5],
                    "something": { "else": "works" }
                }
            },
            {
                "can_be_ordered_differently": {
                    "something": { "else": "works" },
                    "array": [1, 2, 3, 4, 5]
                },
                "nesting": [{ "b": "b", "a": "a" }, { "d": "d", "c": "c" }],
                "name": "this is duplicated"
            },
            {
                "name": "this is unique",
                "nesting": [{ "a": "b", "b": "a" }, { "c": "d", "d": "c" }],
                "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "does not work" }
                }
            },
            {
                "name": "this is unique",
                "nesting": [ { "a": "a", "b": "b", "c": "c" }, { "d": "d", "e": "e", "f": "f" }],
                "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "works" }
                }
            }
        ]
    "#;

    test()
        .run_with_data("$in | from json | uniq | length", sample)
        .expect_value_eq(3)
}

#[test]
fn uniq_when_keys_out_of_order() -> Result {
    let code = r#"
        [
            {"a": "a", "b": [1,2,3]},
            {"b": [1,2,3], "a": "a"}
        ] | uniq | length
    "#;

    test().run(code).expect_value_eq(1)
}

#[rstest]
#[case::a("A", 2)]
#[case::a("B", 1)]
fn uniq_counting(#[case] item: &str, #[case] value: u32) -> Result {
    #[rustfmt::skip]
    let code = r#"
        let item = $in

        ["A", "B", "A"]
        | wrap item
        | uniq --count
        | flatten
        | where item == $item
        | get count
        | get 0
    "#;

    test().run_with_data(code, item).expect_value_eq(value)
}

#[test]
fn uniq_unique() -> Result {
    test()
        .run("[1, 2, 3, 4, 1, 5] | uniq --unique")
        .expect_value_eq([2, 3, 4, 5])
}

#[test]
fn uniq_simple_vals_ints() -> Result {
    test()
        .run("[1, 2, 3, 4, 1, 5] | uniq")
        .expect_value_eq([1, 2, 3, 4, 5])
}

#[test]
fn uniq_simple_vals_strs() -> Result {
    test()
        .run_with_data("$in | uniq", [A, B, C, A])
        .expect_value_eq([A, B, C])
}

#[test]
fn table() -> Result {
    let code = "
        [
            [fruit day];
            [apple monday]
            [apple friday]
            [Apple friday]
            [apple monday]
            [pear monday]
            [orange tuesday]
        ] | uniq
    ";

    let expected = "
        [
            [fruit day];
            [apple monday]
            [apple friday]
            [Apple friday]
            [pear monday]
            [orange tuesday]
        ]
    ";

    let actual: Value = test().run(code)?;
    let expected: Value = test().run(expected)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn keep_last_alone_rarely_changes_output() -> Result {
    // Without --ignore-case, duplicates are already identical values, so keeping
    // the last one looks the same as keeping the first.
    test()
        .run("[1, 2, 1] | uniq --keep-last")
        .expect_value_eq([1, 2])
}

#[test]
fn keep_last_keeps_position_of_first_occurrence() -> Result {
    test()
        .run("['a' 'B' 'A'] | uniq --ignore-case --keep-last")
        .expect_value_eq(["A", "B"])
}

#[test]
fn keep_last_with_count() -> Result {
    let code = "['a' 'B' 'A'] | uniq --ignore-case --keep-last --count | to nuon";
    test()
        .run(code)
        .expect_value_eq("[[value, count]; [A, 2], [B, 1]]")
}

#[test]
fn keep_last_with_repeated() -> Result {
    test()
        .run("['a' 'B' 'A'] | uniq --ignore-case --keep-last --repeated")
        .expect_value_eq(["A"])
}

#[test]
fn keep_last_preserves_position_with_differing_key_order() -> Result {
    // `expect_value_eq` sorts record columns before comparing, which would hide a
    // regression here, so compare the column order itself instead.
    let code = r#"[{a: 1, b: 2}, {b: 2, a: 1}] | uniq --keep-last | get 0 | columns"#;
    test().run(code).expect_value_eq(["b", "a"])
}

#[test]
fn table_with_ignore_case() -> Result {
    let code = "
        [
            [origin, people];
            [World, ([
                [name, meal];
                ['Geremias', {plate: 'bitoque', carbs: 100}]
            ])],
            [World, ([
                [name, meal];
                ['Martin', {plate: 'bitoque', carbs: 100}]
            ])],
            [World, ([
                [name, meal];
                ['Geremias', {plate: 'Bitoque', carbs: 100}]
            ])],
        ] | uniq --ignore-case
    ";

    let expected = "
        [
            [origin, people];
            [World, ([
                [name, meal];
                ['Geremias', {plate: 'bitoque', carbs: 100}]
            ])],
            [World, ([
                [name, meal];
                ['Martin', {plate: 'bitoque', carbs: 100}]
            ])],
        ]
    ";

    let actual: Value = test().run(code)?;
    let expected: Value = test().run(expected)?;
    assert_eq!(actual, expected);
    Ok(())
}
