use nu_test_support::nu;

#[test]
fn chunk_size_negative() {
    let actual = nu!("[0 1 2] | chunks -1");
    assert!(actual.err.contains("positive"));
}

#[test]
fn chunk_size_zero() {
    let actual = nu!("[0 1 2] | chunks 0");
    assert!(actual.err.contains("zero"));
}

#[test]
fn chunk_size_not_int() {
    let actual = nu!("[0 1 2] | chunks (if true { 1sec })");
    assert!(actual.err.contains("can't convert"));
}

#[test]
fn empty() {
    let actual = nu!("[] | chunks 2 | is-empty");
    assert_eq!(actual.out, "true");
}

#[test]
fn list_stream() {
    let actual = nu!("([0 1 2] | every 1 | chunks 2) == ([0 1 2] | chunks 2)");
    assert_eq!(actual.out, "true");
}

#[test]
fn table_stream() {
    let actual = nu!("([[foo bar]; [0 1] [2 3] [4 5]] | every 1 | chunks 2) == ([[foo bar]; [0 1] [2 3] [4 5]] | chunks 2)");
    assert_eq!(actual.out, "true");
}

#[test]
fn no_empty_chunks() {
    let actual = nu!("([0 1 2 3 4 5] | chunks 3 | length) == 2");
    assert_eq!(actual.out, "true");
}
