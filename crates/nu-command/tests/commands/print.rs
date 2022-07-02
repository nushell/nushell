use nu_test_support::{nu, pipeline};

#[test]
fn print_to_stdout() {
    let actual = nu!(
        cwd: ".", pipeline(
            "print 'hello world'"
        )
    );
    assert!(actual.out.contains("hello world"));
    assert!(actual.err.is_empty());
}

#[test]
fn print_to_stderr() {
    let actual = nu!(
        cwd: ".", pipeline(
            "print -e 'hello world'"
        )
    );
    assert!(actual.out.is_empty());
    assert!(actual.err.contains("hello world"));
}
