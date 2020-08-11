use nu_test_support::{nu, pipeline};

#[test]
fn flatten_nested_rows() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo "{"dog_names":{"dog_1":"susan","dog_2":"frank"}}" | from json | flatten dog_names | to json
        "#
    ));

    let flattened_output = r#"{"dog_1":"susan","dog_2":"frank"}"#;

    assert_eq!(actual.out, flattened_output);
}
