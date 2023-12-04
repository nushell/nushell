use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn removes_duplicate_rows() {
    Playground::setup("uniq_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                open los_tres_caballeros.csv
                | uniq
                | length
            "
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn uniq_values() {
    Playground::setup("uniq_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at,type
                Andrés,Robalino,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
                JT,Turner,10/12/2013,B
                Yehuda,Katz,10/11/2013,A
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                open los_tres_caballeros.csv
                | select type
                | uniq
                | length
            "
        ));

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn uniq_empty() {
    let actual = nu!("[] | uniq | to nuon");

    assert_eq!(actual.out, "[]");
}

#[test]
fn nested_json_structures() {
    Playground::setup("uniq_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nested_json_structures.json",
            r#"
            [
                {
                  "name": "this is duplicated",
                  "nesting": [ { "a": "a", "b": "b" },
                               { "c": "c", "d": "d" }
                  ],
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
                  "nesting": [ { "b": "b", "a": "a" },
                               { "d": "d", "c": "c" }
                  ],
                  "name": "this is duplicated"
                },
                {
                  "name": "this is unique",
                  "nesting": [ { "a": "b", "b": "a" },
                               { "c": "d", "d": "c" }
                  ],
                  "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "does not work" }
                  }
                },
                {
                  "name": "this is unique",
                  "nesting": [ { "a": "a", "b": "b", "c": "c" },
                               { "d": "d", "e": "e", "f": "f" }
                  ],
                  "can_be_ordered_differently": {
                    "array": [],
                    "something": { "else": "works" }
                  }
                }
              ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            "
                open nested_json_structures.json
                | uniq
                | length
            "
        ));
        assert_eq!(actual.out, "3");
    })
}

#[test]
fn uniq_when_keys_out_of_order() {
    let actual = nu!(pipeline(
        r#"
            [{"a": "a", "b": [1,2,3]}, {"b": [1,2,3], "a": "a"}]
            | uniq
            | length
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn uniq_counting() {
    let actual = nu!(pipeline(
        r#"
            ["A", "B", "A"]
            | wrap item
            | uniq --count
            | flatten
            | where item == A
            | get count
            | get 0
        "#
    ));
    assert_eq!(actual.out, "2");

    let actual = nu!(pipeline(
        r#"
            ["A", "B", "A"]
            | wrap item
            | uniq --count
            | flatten
            | where item == B
            | get count
            | get 0
        "#
    ));
    assert_eq!(actual.out, "1");
}

#[test]
fn uniq_unique() {
    let actual = nu!("[1 2 3 4 1 5] | uniq --unique");
    let expected = nu!("[2 3 4 5]");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn uniq_simple_vals_ints() {
    let actual = nu!("[1 2 3 4 1 5] | uniq");
    let expected = nu!("[1 2 3 4 5]");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn uniq_simple_vals_strs() {
    let actual = nu!("[A B C A] | uniq");
    let expected = nu!("[A B C]");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn table() {
    let actual = nu!(pipeline(
        "
            [[fruit day]; [apple monday] [apple friday] [Apple friday] [apple monday] [pear monday] [orange tuesday]]
            | uniq
        "
    ));

    let expected = nu!("[[fruit day]; [apple monday] [apple friday] [Apple friday] [pear monday] [orange tuesday]]");
    assert_eq!(actual.out, expected.out);
}

#[test]
fn table_with_ignore_case() {
    let actual = nu!(pipeline(
        r#"
            [[origin, people];
                [World, (
                    [[name, meal];
                        ['Geremias', {plate: 'bitoque', carbs: 100}]
                    ]
                )],
                [World, (
                    [[name, meal];
                        ['Martin', {plate: 'bitoque', carbs: 100}]
                    ]
                )],
                [World, (
                    [[name, meal];
                        ['Geremias', {plate: 'Bitoque', carbs: 100}]
                    ]
                )],
            ] | uniq --ignore-case
        "#
    ));

    let expected = nu!(pipeline(
        r#"
        echo [[origin, people];
                [World, (
                    [[name, meal];
                        ['Geremias', {plate: 'bitoque', carbs: 100}]
                    ]
                )],
                [World, (
                    [[name, meal];
                        ['Martin', {plate: 'bitoque', carbs: 100}]
                    ]
                )],
            ]
        "#
    ));

    assert_eq!(actual.out, expected.out);
}
