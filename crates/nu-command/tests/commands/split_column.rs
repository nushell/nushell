use nu_protocol::record;
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn split_column() -> Result {
    Playground::setup("split_column_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed(
                "sample.txt",
                "
                    importer,shipper,tariff_item,name,origin
                ",
            ),
            FileWithContentToBeTrimmed(
                "sample2.txt",
                "
                    importer , shipper  , tariff_item  ,   name  ,  origin
                ",
            ),
        ]);

        let code = r#"
            open sample.txt
            | lines
            | str trim
            | split column ","
            | get column1
        "#;
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(["shipper"])?;

        let code = r#"
            open sample.txt
            | lines
            | str trim
            | split column -n 3 ","
            | get column2
        "#;
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(["tariff_item,name,origin"])?;

        let code = r#"
            open sample.txt
            | lines
            | str trim
            | split column -n 3 --right ","
            | get column0
        "#;
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(["importer,shipper,tariff_item"])?;

        let code = r"
            open sample2.txt
            | lines
            | str trim
            | split column --regex '\s*,\s*'
            | get column1
        ";
        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq(["shipper"])?;

        Ok(())
    })
}

#[test]
fn split_column_no_sep_found() -> Result {
    let sample = "stuff";
    let expected = vec![Value::test_record(
        record! { "column0" => Value::test_string(sample) },
    )];

    let code = r#"$in | split column "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(expected.clone())?;

    let code = r#"$in | split column -n 3 "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(expected.clone())?;

    let code = r#"$in | split column -n 3 --right "|""#;
    test()
        .run_with_data(code, sample)
        .expect_value_eq(expected.clone())?;
    Ok(())
}

#[test]
fn split_column_number_zero() -> Result {
    let code = r#""x" | split column -n 0 ",""#;
    let expected = vec![Value::test_record(record! {})];
    test().run(code).expect_value_eq(expected.clone())?;
    let code = r#""x" | split column -n 0 --right ",""#;
    test().run(code).expect_value_eq(expected.clone())?;
    Ok(())
}

#[test]
fn split_column_number_error() -> Result {
    let code = r#""x" | split column -n -1 ",""#;
    test().run(code).expect_shell_error()?;
    let code = r#""x" | split column -n -1 --right ",""#;
    test().run(code).expect_shell_error()?;
    Ok(())
}
