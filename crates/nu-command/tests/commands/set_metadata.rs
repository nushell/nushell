use nu_test_support::{nu, pipeline};

#[test]
fn let_with_metadata_reconstruct() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let metadata = (ls | metadata --data);
        $metadata.data | set-metadata $metadata | metadata | get source
        "#
    ));

    assert_eq!(actual.out, "ls");
}
