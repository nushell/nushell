use nu_test_support::nu;

#[test]
fn int() {
    let actual = nu!("1 | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn float() {
    let actual = nu!("1.2 | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn str() {
    let actual = nu!("'2000' | into filesize");
    assert!(actual.out.contains("2.0 kB"));
}

#[test]
fn str_newline() {
    let actual = nu!(r#"
    "2000
    " | into filesize
    "#);

    assert!(actual.out.contains("2.0 kB"));
}

#[test]
fn str_many_newlines() {
    let actual = nu!(r#"
    "2000
    
    " | into filesize
    "#);

    assert!(actual.out.contains("2.0 kB"));
}

#[test]
fn filesize() {
    let actual = nu!("3kB | into filesize");

    assert!(actual.out.contains("3.0 kB"));
}

#[test]
fn negative_filesize() {
    let actual = nu!("-3kB | into filesize");

    assert!(actual.out.contains("-3.0 kB"));
}

#[test]
fn negative_str_filesize() {
    let actual = nu!("'-3kB' | into filesize");

    assert!(actual.out.contains("-3.0 kB"));
}

#[test]
fn wrong_negative_str_filesize() {
    let actual = nu!("'--3kB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn large_negative_str_filesize() {
    let actual = nu!("'-10000PB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn negative_str() {
    let actual = nu!("'-1' | into filesize");

    assert!(actual.out.contains("-1 B"));
}

#[test]
fn wrong_negative_str() {
    let actual = nu!("'--1' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn positive_str_filesize() {
    let actual = nu!("'+1kB' | into filesize");

    assert!(actual.out.contains("1.0 kB"));
}

#[test]
fn wrong_positive_str_filesize() {
    let actual = nu!("'++1kB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn large_positive_str_filesize() {
    let actual = nu!("'+10000PB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn positive_str() {
    let actual = nu!("'+1' | into filesize");

    assert!(actual.out.contains("1 B"));
}

#[test]
fn wrong_positive_str() {
    let actual = nu!("'++1' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}

#[test]
fn invalid_str() {
    let actual = nu!("'42.0 42.0 kB' | into filesize");

    assert!(actual.err.contains("can't convert string to filesize"));
}
