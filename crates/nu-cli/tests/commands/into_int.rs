use nu_test_support::{nu, pipeline};

#[test]
fn into_int_filesize() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        into-int 1kb | each {= $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        into-int 1024 | each {= $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}
