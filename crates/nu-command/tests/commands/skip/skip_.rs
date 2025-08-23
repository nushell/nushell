use nu_test_support::nu;

#[test]
fn skips_bytes() {
    let actual = nu!("(0x[aa bb cc] | skip 2) == 0x[cc]");

    assert_eq!(actual.out, "true");
}

#[test]
fn skips_bytes_from_stream() {
    let actual = nu!("([0 1] | each { 0x[aa bb cc] } | bytes collect | skip 2) == 0x[cc aa bb cc]");

    assert_eq!(actual.out, "true");
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | skip 2");

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn skips_bytes_and_drops_content_type() {
    let actual = nu!(format!(
        "open {} | skip 3 | metadata | get content_type? | describe",
        file!(),
    ));
    assert_eq!(actual.out, "nothing");
}
