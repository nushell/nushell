use nu_test_support::nu;

#[test]
fn test_1() {
    let actual = nu!(r#"
    echo 1..5 | into string | str join
    "#);

    assert_eq!(actual.out, "12345");
}

#[test]
fn test_2() {
    let actual = nu!(r#"
    echo [a b c d] | str join "<sep>"
    "#);

    assert_eq!(actual.out, "a<sep>b<sep>c<sep>d");
}

#[test]
fn test_stream() {
    let actual = nu!("[a b c d] | filter {true} | str join .");
    assert_eq!(actual.out, "a.b.c.d");
}

#[test]
fn test_stream_type() {
    let actual = nu!("[a b c d] | filter {true} | str join . | describe -n");
    assert_eq!(actual.out, "string (stream)");
}

#[test]
fn construct_a_path() {
    let actual = nu!(r#"
    echo [sample txt] | str join "."
    "#);

    assert_eq!(actual.out, "sample.txt");
}
