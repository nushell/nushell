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

    assert_eq!(actual.out, "first second");
}

#[test]
fn with_env_shorthand_trims_quotes() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "FOO='BARRRR' echo $nu.env | get FOO"
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_and_shorthand_same_result() {
    let actual_shorthand = nu!(
        cwd: "tests/fixtures/formats",
        "FOO='BARRRR' echo $nu.env | get FOO"
    );

    let actual_normal = nu!(
        cwd: "tests/fixtures/formats",
        "with-env [FOO BARRRR] {echo $nu.env} | get FOO"
    );

    assert_eq!(actual_shorthand.out, actual_normal.out);
}

#[test]
fn with_env_shorthand_nested_quotes() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "FOO='-arg \"hello world\"' echo $nu.env | get FOO"
    );

    assert_eq!(actual.out, "-arg \"hello world\"");
}
