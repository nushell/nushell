use nu_test_support::{nu, pipeline};

#[test]
fn view_source_alias_inside_closure() {
    let actual = nu!(pipeline(
        r#"
            do { alias a = print; a 'alias is alive'; view source a }
        "#
    ));

    assert_eq!(actual.out, "alias is aliveprint");
    assert!(actual.err.is_empty());
}
