use nu_test_support::{nu, pipeline};

#[test]
fn into_filesize_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        1 | into filesize
        "#
    ));

    assert!(actual.out.contains("1 B"));
}

#[test]
fn into_filesize_decimal() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        1.2 | into filesize
        "#
    ));

    assert!(actual.out.contains("1 B"));
}

#[test]
fn into_filesize_str() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        '2000' | into filesize
        "#
    ));

<<<<<<< HEAD
    assert!(actual.out.contains("2.0 KB"));
=======
    assert!(actual.out.contains("2.0 KiB"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn into_filesize_str_newline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        '2000
' | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KB"));
=======
        "2000
" | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KiB"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn into_filesize_str_many_newlines() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        '2000

' | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KB"));
=======
        "2000

" | into filesize
        "#
    ));

    assert!(actual.out.contains("2.0 KiB"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn into_filesize_filesize() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
<<<<<<< HEAD
        3kb | into filesize
        "#
    ));

    assert!(actual.out.contains("3.0 KB"));
=======
        3kib | into filesize
        "#
    ));

    assert!(actual.out.contains("3.0 KiB"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
