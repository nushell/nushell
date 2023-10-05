use nu_test_support::nu;

#[test]
fn url_decode_simple() {
    let actual = nu!(r#"'a%20b' | url decode"#);
    assert_eq!(actual.out, "a b");
}

#[test]
fn url_decode_special_characters() {
    let actual = nu!(r#"'%21%40%23%24%25%C2%A8%26%2A%2D%2B%3B%2C%7B%7D%5B%5D%28%29' | url decode"#);
    assert_eq!(actual.out, r#"!@#$%¨&*-+;,{}[]()"#);
}

#[test]
fn url_decode_error_invalid_utf8() {
    let actual = nu!(r#"'%99' | url decode"#);
    assert!(actual.err.contains("invalid utf-8 sequence"));
}
