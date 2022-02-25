use nu_test_support::nu;

#[test]
fn with_env_extends_environment() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "with-env [FOO BARRRR] {echo $env} | get FOO"
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_shorthand() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "FOO=BARRRR echo $env | get FOO"
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
        "FOO='BARRRR' echo $env | get FOO"
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_and_shorthand_same_result() {
    let actual_shorthand = nu!(
        cwd: "tests/fixtures/formats",
        "FOO='BARRRR' echo $env | get FOO"
    );

    let actual_normal = nu!(
        cwd: "tests/fixtures/formats",
        "with-env [FOO BARRRR] {echo $env} | get FOO"
    );

    assert_eq!(actual_shorthand.out, actual_normal.out);
}

#[test]
fn test_redirection2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "let x = (FOO=BAR nu --testbin cococo niceenvvar); $x | str trim | str length"
    );

    assert_eq!(actual.out, "10");
}

#[test]
fn with_env_hides_variables_in_parent_scope() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        let-env FOO = "1"
        echo $env.FOO
        with-env [FOO $nothing] {
            echo $env.FOO
        }
        echo $env.FOO
        "#
    );

    assert_eq!(actual.out, "11");
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn with_env_shorthand_can_not_hide_variables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        let-env FOO = "1"
        echo $env.FOO
        FOO=$nothing echo $env.FOO
        echo $env.FOO
        "#
    );

    assert_eq!(actual.out, "1$nothing1");
}
