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

#[test]
fn works_with_closure_setting_content_type() {
    let actual = nu!(
        cwd: ".",
        r#"
        "data" | metadata set {|meta| {content_type: "text/plain"}} | metadata | get content_type
        "#
    );

    assert_eq!(actual.out, "text/plain");
}

#[test]
fn works_with_closure_modifying_existing_metadata() {
    let actual = nu!(
        cwd: ".",
        r#"
        "data" | metadata set --content-type "text/csv" | metadata set {|meta| {content_type: ($meta.content_type + "-modified")}} | metadata | get content_type
        "#
    );

    assert_eq!(actual.out, "text/csv-modified");
}

#[test]
fn works_with_closure_using_in() {
    let actual = nu!(
        cwd: ".",
        r#"
        "data" | metadata set --content-type "text/html" | metadata set {|| {content_type: ($in.content_type | str replace "html" "xml")}} | metadata | get content_type
        "#
    );

    assert_eq!(actual.out, "text/xml");
}

#[test]
fn closure_can_set_custom_metadata() {
    let actual = nu!(
        cwd: ".",
        r#"
        "data" | metadata set {|meta| {custom_key: "custom_value", another: 42}} | metadata | get custom_key
        "#
    );

    assert_eq!(actual.out, "custom_value");
}

#[test]
fn errors_when_closure_with_datasource_ls() {
    let actual = nu!(r#"
    echo "foo" | metadata set {|meta| {content_type: "text/plain"}} --datasource-ls
    "#);

    assert!(actual.err.contains("cannot use closure with other flags"));
}

#[test]
fn errors_when_closure_with_datasource_filepath() {
    let actual = nu!(r#"
    echo "foo" | metadata set {|meta| {content_type: "text/plain"}} --datasource-filepath foo.txt
    "#);

    assert!(actual.err.contains("cannot use closure with other flags"));
}

#[test]
fn errors_when_closure_with_content_type() {
    let actual = nu!(r#"
    echo "foo" | metadata set {|meta| {content_type: "text/plain"}} --content-type "text/csv"
    "#);

    assert!(actual.err.contains("cannot use closure with other flags"));
}

#[test]
fn errors_when_closure_with_merge() {
    let actual = nu!(r#"
    echo "foo" | metadata set {|meta| {content_type: "text/plain"}} --merge {custom: "value"}
    "#);

    assert!(actual.err.contains("cannot use closure with other flags"));
}

#[test]
fn errors_when_closure_returns_non_record() {
    let actual = nu!(r#"
    echo "foo" | metadata set {|meta| "not a record"}
    "#);

    assert!(actual.err.contains("Closure must return a record"));
}
