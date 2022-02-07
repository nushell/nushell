use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn to_row() {
    Playground::setup("split_row_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "sample.txt",
            r#"
                importer,shipper,tariff_item,name,origin
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open sample.txt
                | lines
                | str trim
                | split row ","
                | length
            "#
        ));

        assert!(actual.out.contains('5'));
    })
}
