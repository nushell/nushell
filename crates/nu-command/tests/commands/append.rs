use nu_test_support::prelude::*;

#[test]
fn adds_a_row_to_the_end() -> Result {
    test()
        .run(
            r#"echo [ "Andrés N. Robalino", "JT Turner", "Yehuda Katz" ] | append "pollo loco" | get 3"#,
        )
        .expect_value_eq("pollo loco")
}
