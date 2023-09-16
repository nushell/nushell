use nu_test_support::{nu, pipeline};

#[test]
fn into_filesize_int() {
    let actual = nu!("1 | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn into_filesize_float() {
    let actual = nu!("1.2 | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn into_filesize_str() {
    let actual = nu!(r#"
        '2000' | into filesize
        "#);

    assert!(actual.out.contains("2.0 KiB"));
}

#[test]
fn into_filesize_str_newline() {
    let actual = nu!(pipeline(
        r#"
        "2000
" | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KiB"));
}

#[test]
fn into_filesize_str_many_newlines() {
    let actual = nu!(pipeline(
        r#"
        "2000

" | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KiB"));
}

#[test]
fn into_filesize_filesize() {
    let actual = nu!("3kib | into filesize");

    assert!(actual.out.contains("3.0 KiB"));
}

#[test]
fn into_filesize_negative_filesize() {
    let actual = nu!("-3kib | into filesize");

    assert!(actual.out.contains("-3.0 KiB"));
}
