use nu_test_support::prelude::*;

#[test]
fn to_row() -> Result {
    let sample = "importer,shipper,tariff_item,name,origin";
    let code = r#"$in | split row "," | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)?;

    let sample = "importer      ,   shipper      ,  tariff_item,name      ,    origin";
    let code = r#"$in | split row -r '\s*,\s*' | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)?;

    let code = r#"
        def foo [a: list<string>] {
            $a | describe
        }

        foo (["a b", "c d"] | split row " ")
    "#;
    test().run(code).expect_value_eq("list<string>")
}
