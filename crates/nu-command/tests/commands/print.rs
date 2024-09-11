use nu_test_support::nu;

#[test]
fn print_to_stdout() {
    let actual = nu!("print 'hello world'");
    assert!(actual.out.contains("hello world"));
    assert!(actual.err.is_empty());
}

#[test]
fn print_to_stderr() {
    let actual = nu!("print -e 'hello world'");
    assert!(actual.out.is_empty());
    assert!(actual.err.contains("hello world"));
}

#[test]
fn print_raw() {
    let actual = nu!("0x[41 42 43] | print --raw");
    assert_eq!(actual.out, "ABC");
}

#[test]
fn print_raw_stream() {
    let actual = nu!("[0x[66] 0x[6f 6f]] | bytes collect | print --raw");
    assert_eq!(actual.out, "foo");
}
