use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn view_source_returns_string() {
    let actual = nu!(r#"def foo [] { echo hi }; view source foo"#);
    assert_eq!(actual.out, "def foo [] { echo hi }");
}

#[test]
fn datasource_filepath_metadata() {
    Playground::setup("cd_ds_filepath_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "mdata.nu",
            r#"
                def foo [] { echo hi }
            "#,
        )]);
        let actual = nu!(
            cwd: dirs.test(),
            r#"
        source mdata.nu
        view source foo | metadata | get source
        "#
        );
        // expect path printed somehow
        assert!(actual.out.contains("mdata.nu"));
    })
}
