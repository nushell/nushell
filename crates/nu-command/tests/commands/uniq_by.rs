use std::collections::HashMap;

use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn removes_duplicate_rows() -> Result {
    let sample = r#"[
        [first_name, last_name, rusty_at,     type];
        [Andrés    , Robalino,  "10/11/2013", A   ],
        [Afonso    , Turner,    "10/12/2013", B   ],
        [Yehuda    , Katz,      "10/11/2013", A   ],
        [JT        , Turner,    "11/12/2011", O   ]
    ]"#;

    let code = format!("{sample} | uniq-by last_name | length");
    test().run(code).expect_value_eq(3)
}

#[test]
fn uniq_when_keys_out_of_order() -> Result {
    let code = r#"[{"a": "a", "b": [1,2,3]}, {"b": [1,2,3,4], "a": "a"}] | uniq-by a"#;
    let (outcome,): (HashMap<String, Value>,) = test().run(code)?;
    outcome["a"].assert_eq("a");
    outcome["b"].assert_eq([1, 2, 3]);
    Ok(())
}

#[rstest]
#[case::a("A", 2)]
#[case::b("B", 1)]
fn uniq_counting(#[case] item: &str, #[case] count: u32) -> Result {
    #[rustfmt::skip]
    let code = format!(r#"
        ["A", "B", "A"]
        | wrap item
        | uniq-by item --count
        | flatten
        | where item == {item}
        | get count
        | get 0
    "#);

    test().run(code).expect_value_eq(count)
}

#[test]
fn uniq_unique() -> Result {
    let code = "
        echo [1 2 3 4 1 5]
        | wrap item
        | uniq-by item --unique
        | get item
    ";

    test().run(code).expect_value_eq([2, 3, 4, 5])
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
        ]
        | uniq-by fruit
    ";

    let expected = "
        [
            [fruit day];
            [apple monday]
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
fn uniq_by_empty() -> Result {
    test()
        .run("[] | uniq-by foo | to nuon")
        .expect_value_eq("[]")
}

#[test]
fn uniq_by_multiple_columns() -> Result {
    let code = "
        [
            [fruit day];
            [apple monday]
            [apple friday]
            [Apple friday]
            [apple monday]
            [pear monday]
            [orange tuesday]
        ]
        | uniq-by fruit day
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
        ] | uniq-by people -i
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

#[test]
fn missing_parameter() -> Result {
    let err = test().run("[11 22 33] | uniq-by").expect_shell_error()?;
    assert!(matches!(err, ShellError::MissingParameter { .. }));
    Ok(())
}

#[test]
fn wrong_column() -> Result {
    let err = test()
        .run("[[fruit day]; [apple monday] [apple friday]] | uniq-by column1")
        .expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "column1");
            Ok(())
        }
        err => Err(err.into()),
    }
}
