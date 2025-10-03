use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn to_column() {
    Playground::setup("split_column_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed(
                "sample.txt",
                r#"
                importer,shipper,tariff_item,name,origin
            "#,
            ),
            FileWithContentToBeTrimmed(
                "sample2.txt",
                r#"
                importer , shipper  , tariff_item  ,   name  ,  origin
            "#,
            ),
        ]);

        let actual = nu!(cwd: dirs.test(), r#"
            open sample.txt
            | lines
            | str trim
            | split column ","
            | get column2
        "#);

        assert!(actual.out.contains("shipper"));

        let actual = nu!(cwd: dirs.test(), r#"
            open sample.txt
            | lines
            | str trim
            | split column -n 3 ","
            | get column3
        "#);

        assert!(actual.out.contains("tariff_item,name,origin"));

        let actual = nu!(cwd: dirs.test(), r"
            open sample2.txt
            | lines
            | str trim
            | split column --regex '\s*,\s*'
            | get column2
        ");

        assert!(actual.out.contains("shipper"));
    })
}
