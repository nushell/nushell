use nu_test_support::nu;

#[test]
fn list_to_text() {
    // Using `str length` since nu! strips newlines, grr
    let actual = nu!(r#"[] | to text | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | to text | str length"#);
    assert_eq!(actual.out, "2");

    let actual = nu!(r#"[a b] | to text | str length"#);
    assert_eq!(actual.out, "4");
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[test]
fn list_stream_to_text() {
    let actual = nu!(r#"[] | each {} | to text | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | each {} | to text | str length"#);
    assert_eq!(actual.out, "2");

    let actual = nu!(r#"[a b] | each {} | to text | str length"#);
    assert_eq!(actual.out, "4");
}

#[test]
fn list_to_text_no_newline() {
    // Using `str length` since nu! strips newlines, grr
    let actual = nu!(r#"[] | to text -n | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | to text -n | str length"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"[a b] | to text -n | str length"#);
    assert_eq!(actual.out, "3");
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[test]
fn list_stream_to_text_no_newline() {
    let actual = nu!(r#"[] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"[a b] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, "3");
}
