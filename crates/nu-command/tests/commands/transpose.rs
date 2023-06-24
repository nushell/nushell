use nu_test_support::nu;

#[test]
fn row() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r | debug");

    assert!(actual.out.contains("foo: 1"));
}

#[test]
fn row_but_last() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r -l | debug");

    assert!(actual.out.contains("foo: 2"));
}

#[test]
fn row_but_all() {
    let actual = nu!("[[key value]; [foo 1] [foo 2]] | transpose -r -a | debug");

    assert!(actual.out.contains("foo: [1, 2]"));
}
