use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn condition_is_met() -> Result {
    let sample = r#"[
        ["Chicken Collection", "29/04/2020", "30/04/2020", "31/04/2020"];
        ["Yellow Chickens", "", "", ""],
        [Andrés, 1, 1, 1],
        [JT, 1, 1, 1],
        [Jason, 1, 1, 1],
        [Yehuda, 1, 1, 1],
        ["Blue Chickens", "", "", ""],
        [Andrés, 1, 1, 2],
        [JT, 1, 1, 2],
        [Jason, 1, 1, 2],
        [Yehuda, 1, 1, 2],
        ["Red Chickens", "", "", ""],
        [Andrés, 1, 1, 3],
        [JT, 1, 1, 3],
        [Jason, 1, 1, 3],
        [Yehuda, 1, 1, 3]
    ]"#;

    let code = r#"
        from nuon
        | skip 1
        | take while {|row| $row."Chicken Collection" != "Blue Chickens"}
        | into int "31/04/2020"
        | get "31/04/2020"
        | math sum
    "#;

    test().run_with_data(code, sample).expect_value_eq(4)
}

#[test]
fn fail_on_non_iterator() -> Result {
    let code = "1 | take while {|row| $row == 2}";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch(..)));
    Ok(())
}

#[rstest]
#[case::none(0, ["a", "b", "c"])]
#[case::one(1, ["a", "b", "c", "d"])]
#[case::two(2, ["a", "b", "c", "d", "e"])]
#[case::more_than_input(12, ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"])]
fn include(#[case] n: i64, #[case] expect: impl IntoValue) -> Result {
    let code = r#"
        let n = $in

        [a b c d e f g h i j]
        | take while -i $n {|e| $e != "d" }
    "#;
    test().run_with_data(code, n).expect_value_eq(expect)
}
