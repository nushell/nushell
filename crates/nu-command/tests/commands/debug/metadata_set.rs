use nu_test_support::nu;

#[test]
fn errors_on_conflicting_metadata_flags() {
    let actual = nu!(r#"
    echo "foo" | metadata set --datasource-filepath foo.txt --datasource-ls
    "#);

    assert!(actual.err.contains("cannot use `--datasource-filepath`"));
    assert!(actual.err.contains("with `--datasource-ls`"));
}

#[test]
fn works_with_datasource_filepath() {
    let actual = nu!(r#"
    echo "foo"
    | metadata set --datasource-filepath foo.txt
    | metadata
    "#);

    assert!(actual.out.contains("foo.txt"));
}

#[test]
fn works_with_datasource_ls() {
    let actual = nu!(r#"
    echo "foo"
    | metadata set --datasource-ls
    | metadata
    "#);

    assert!(actual.out.contains("ls"));
}

#[test]
fn works_with_merge_arbitrary_metadata() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo "foo"
        | metadata set --merge {custom_key: "custom_value", foo: 42}
        | metadata
        | get custom_key
        "#
    );

    assert_eq!(actual.out, "custom_value");
}

#[test]
fn merge_preserves_existing_metadata() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo "foo"
        | metadata set --content-type "text/plain"
        | metadata set --merge {custom: "value"}
        | metadata
        | get content_type
        "#
    );

    assert_eq!(actual.out, "text/plain");
}

#[test]
fn custom_metadata_preserved_through_collect() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo "foo"
        | metadata set --merge {custom_key: "custom_value"}
        | collect
        | metadata
        | get custom_key
        "#
    );

    assert_eq!(actual.out, "custom_value");
}
