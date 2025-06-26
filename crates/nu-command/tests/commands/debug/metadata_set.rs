use nu_test_support::nu;
use nu_test_support::pipeline;

#[test]
fn errors_on_conflicting_metadata_flags() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "foo" | metadata set --datasource-filepath foo.txt --datasource-ls
        "#
    ));

    assert!(
        actual
            .err
            .contains("Cannot use both --datasource-filepath and --datasource-ls")
    );
}

#[test]
fn works_with_datasource_filepath() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "foo" | metadata set --datasource-filepath foo.txt
        "#
    ));

    assert!(actual.out.contains("foo"));
}
