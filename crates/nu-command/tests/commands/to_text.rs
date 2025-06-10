use nu_test_support::nu;

const LINE_LEN: usize = if cfg!(target_os = "windows") { 2 } else { 1 };

#[test]
fn list() {
    // Using `str length` since nu! strips newlines, grr
    let actual = nu!(r#"[] | to text | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | to text | str length"#);
    assert_eq!(actual.out, (1 + LINE_LEN).to_string());

    let actual = nu!(r#"[a b] | to text | str length"#);
    assert_eq!(actual.out, (2 * (1 + LINE_LEN)).to_string());
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[test]
fn list_stream() {
    let actual = nu!(r#"[] | each {} | to text | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | each {} | to text | str length"#);
    assert_eq!(actual.out, (1 + LINE_LEN).to_string());

    let actual = nu!(r#"[a b] | each {} | to text | str length"#);
    assert_eq!(actual.out, (2 * (1 + LINE_LEN)).to_string());
}

#[test]
fn list_no_newline() {
    let actual = nu!(r#"[] | to text -n | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | to text -n | str length"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"[a b] | to text -n | str length"#);
    assert_eq!(actual.out, (2 + LINE_LEN).to_string());
}

// The output should be the same when `to text` gets a ListStream instead of a Value::List.
#[test]
fn list_stream_no_newline() {
    let actual = nu!(r#"[] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, "0");

    let actual = nu!(r#"[a] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"[a b] | each {} | to text -n | str length"#);
    assert_eq!(actual.out, (2 + LINE_LEN).to_string());
}
