use nu_test_support::nu;

#[test]
fn with_env_extends_environment() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "with-env [FOO BARRRR] {echo $nu.env} | get FOO"
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_shorthand() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "FOO=BARRRR echo $nu.env | get FOO"
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn shorthand_doesnt_reorder_arguments() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "FOO=BARRRR nu --testbin cococo first second"
    );

    assert_eq!(actual.out, "firstsecond");
}
