use nu_test_support::nu;

#[test]
fn timeit_show_stdout() {
    let actual = nu!("let t = timeit { nu --testbin cococo abcdefg }");
    assert_eq!(actual.out, "abcdefg");
}

#[test]
fn timeit_show_stderr() {
    let actual = nu!(
        " with-env {FOO: bar, FOO2: baz} { let t = timeit { nu --testbin echo_env_mixed out-err FOO FOO2 } }"
    );
    assert!(actual.out.contains("bar"));
    assert!(actual.err.contains("baz"));
}

#[test]
fn timeit_show_output() {
    let actual = nu!("timeit --output { 'this is a test' } | get output");
    assert_eq!(actual.out, "this is a test");
}
