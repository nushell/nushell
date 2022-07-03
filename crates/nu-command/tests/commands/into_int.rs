use nu_test_support::{nu, pipeline};

#[test]
fn into_int_filesize() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1kb | into int | each { |it| $it / 1000 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_filesize2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1kib | into int | each { |it| $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1024 | into int | each { |it| $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_binary() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 0x[01010101] | into int
        "#
    ));

    assert!(actual.out.contains("16843009"));
}
