use nu_test_support::prelude::*;

#[test]
fn adds_a_row_to_the_beginning() -> Result {
    let input = ["Andrés N. Robalino", "JT Turner", "Yehuda Katz"];
    let code = r#"$in | prepend "pollo loco" | get 0"#;
    test()
        .run_with_data(code, input)
        .expect_value_eq("pollo loco")
}
