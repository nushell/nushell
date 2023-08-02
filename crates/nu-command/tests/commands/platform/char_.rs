use nu_test_support::{nu, pipeline};

#[test]
fn test_char_list_outputs_table() {
    let actual = nu!(pipeline(
        r#"
            char --list | length
        "#
    ));

    assert_eq!(actual.out, "107");
}
