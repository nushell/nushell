use nu_test_support::nu;

#[test]
fn condition_is_met() {
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

    let actual = nu!(format!(
        r#"
            {sample}
            | skip while {{|row| $row."Chicken Collection" != "Blue Chickens" }}
            | take until {{|row| $row."Chicken Collection" == "Red Chickens" }}
            | skip 1
            | into int "31/04/2020"
            | get "31/04/2020"
            | math sum
        "#
    ));

    assert_eq!(actual.out, "8");
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | take until {|row| $row == 2}");

    assert!(actual.err.contains("command doesn't support"));
}
