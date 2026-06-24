use nu_test_support::prelude::*;

#[test]
fn split_row() -> Result {
    let sample = "importer,shipper,tariff_item,name,origin";
    let code = r#"$in | split row "," | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)
}

#[test]
fn split_row_number() -> Result {
    let sample = "importer,shipper,tariff_item,name,origin";

    let code = r#"$in | split row -n 3 ",""#;
    test().run_with_data(code, sample).expect_value_eq(vec![
        "importer",
        "shipper",
        "tariff_item,name,origin",
    ])?;

    let code = r#"$in | split row -n 3 --right ",""#;
    test().run_with_data(code, sample).expect_value_eq(vec![
        "importer,shipper,tariff_item",
        "name",
        "origin",
    ])
}

#[test]
fn split_row_number_zero() -> Result {
    let code = r#""x" | split row -n 0 ",""#;
    test().run(code).expect_value_eq::<Vec<Value>>(vec![])?;
    let code = r#""x" | split row -n 0 --right ",""#;
    test().run(code).expect_value_eq::<Vec<Value>>(vec![])?;
    Ok(())
}

#[test]
fn split_row_number_error() -> Result {
    let code = r#""x" | split row -n -1 ",""#;
    test().run(code).expect_shell_error()?;
    let code = r#""x" | split row -n -1 --right ",""#;
    test().run(code).expect_shell_error()?;
    Ok(())
}

#[test]
fn to_row_no_sep_found() -> Result {
    let sample = "stuff";

    let code = r#"$in | split row "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])?;

    let code = r#"$in | split row -n 3 "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])?;

    let code = r#"$in | split row -n 3 --right "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(vec![sample])
}

#[test]
fn split_row_regex() -> Result {
    let sample = "importer      ,   shipper      ,  tariff_item,name      ,    origin";
    let code = r#"$in | split row -r '\s*,\s*' | length"#;
    test().run_with_data(code, sample).expect_value_eq(5)
}

#[test]
fn split_row_type() -> Result {
    let code = r#"
        def foo [a: list<string>] {
            $a | describe
        }

        foo (["a b", "c d"] | split row " ")
    "#;
    test().run(code).expect_value_eq("list<string>")
}
