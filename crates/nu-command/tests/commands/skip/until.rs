use nu_test_support::prelude::*;

#[test]
fn condition_is_met() -> Result {
    let sample = r#"[
        ["Chicken Collection", "29/04/2020", "30/04/2020", "31/04/2020"];
        ["Yellow Chickens", "", "", ""],
        [Andrés, 0, 0, 1],
        [JT, 0, 0, 1],
        [Jason, 0, 0, 1],
        [Yehuda, 0, 0, 1],
        ["Blue Chickens", "", "", ""],
        [Andrés, 0, 0, 2],
        [JT, 0, 0, 2],
        [Jason, 0, 0, 2],
        [Yehuda, 0, 0, 2],
        ["Red Chickens", "", "", ""],
        [Andrés, 0, 0, 1],
        [JT, 0, 0, 1],
        [Jason, 0, 0, 1],
        [Yehuda, 0, 0, 3]
    ]"#;

    let code = format!(
        r#"
            {sample}
            | skip until {{|row| $row."Chicken Collection" == "Red Chickens" }}
            | skip 1
            | into int "31/04/2020"
            | get "31/04/2020"
            | math sum
        "#
    );

    test().run(code).expect_value_eq(6)
}

#[test]
fn fail_on_non_iterator() -> Result {
    let code = "1 | skip until {|row| $row == 2}";
    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}
