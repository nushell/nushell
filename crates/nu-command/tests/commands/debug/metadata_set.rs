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

    assert!(actual.err.contains("cannot use `--datasource-filepath`"));
    assert!(actual.err.contains("with `--datasource-ls`"));
}

#[test]
fn works_with_datasource_filepath() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "foo"
        | metadata set --datasource-filepath foo.txt
        | metadata
        "#
    ));

    assert!(actual.out.contains("foo.txt"));
}

#[test]
fn works_with_datasource_ls() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "foo"
        | metadata set --datasource-ls
        | metadata
        "#
    ));

    assert!(actual.out.contains("ls"));
}
