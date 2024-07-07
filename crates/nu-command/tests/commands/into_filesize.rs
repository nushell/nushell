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

#[test]
fn into_filesize_negative_str_filesize() {
    let actual = nu!("'-3kib' | into filesize");

    assert!(actual.out.contains("-3.0 KiB"));
}

#[test]
fn into_filesize_wrong_negative_str_filesize() {
    let actual = nu!("'--3kib' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn into_filesize_large_negative_str_filesize() {
    let actual = nu!("'-10000PiB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn into_filesize_negative_str() {
    let actual = nu!("'-1' | into filesize");

    assert!(actual.out.contains("-1 B"));
}

#[test]
fn into_filesize_wrong_negative_str() {
    let actual = nu!("'--1' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn into_filesize_positive_str_filesize() {
    let actual = nu!("'+1Kib' | into filesize");

    assert!(actual.out.contains("1.0 KiB"));
}

#[test]
fn into_filesize_wrong_positive_str_filesize() {
    let actual = nu!("'++1Kib' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn into_filesize_large_positive_str_filesize() {
    let actual = nu!("'+10000PiB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn into_filesize_positive_str() {
    let actual = nu!("'+1' | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn into_filesize_wrong_positive_str() {
    let actual = nu!("'++1' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}
