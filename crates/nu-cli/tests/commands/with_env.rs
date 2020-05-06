use nu_test_support::nu;

#[test]
fn with_env_extends_environment() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "with-env [FOO BARRRR] {echo $nu.env} | get FOO"
    );

    assert_eq!(actual, "BARRRR");
}
