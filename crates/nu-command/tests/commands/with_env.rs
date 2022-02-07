use nu_test_support::nu;

#[test]
fn with_env_extends_environment() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
<<<<<<< HEAD
        "with-env [FOO BARRRR] {echo $nu.env} | get FOO"
=======
        "with-env [FOO BARRRR] {echo $env} | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_shorthand() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
<<<<<<< HEAD
        "FOO=BARRRR echo $nu.env | get FOO"
=======
        "FOO=BARRRR echo $env | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
<<<<<<< HEAD
        "FOO='BARRRR' echo $nu.env | get FOO"
=======
        "FOO='BARRRR' echo $env | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    );

    assert_eq!(actual.out, "BARRRR");
}

#[test]
fn with_env_and_shorthand_same_result() {
    let actual_shorthand = nu!(
        cwd: "tests/fixtures/formats",
<<<<<<< HEAD
        "FOO='BARRRR' echo $nu.env | get FOO"
=======
        "FOO='BARRRR' echo $env | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    );

    let actual_normal = nu!(
        cwd: "tests/fixtures/formats",
<<<<<<< HEAD
        "with-env [FOO BARRRR] {echo $nu.env} | get FOO"
=======
        "with-env [FOO BARRRR] {echo $env} | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    );

    assert_eq!(actual_shorthand.out, actual_normal.out);
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn with_env_shorthand_nested_quotes() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
<<<<<<< HEAD
        "FOO='-arg \"hello world\"' echo $nu.env | get FOO"
=======
        "FOO='-arg \"hello world\"' echo $env | get FOO"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    );

    assert_eq!(actual.out, "-arg \"hello world\"");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn with_env_hides_variables_in_parent_scope() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        let-env FOO = "1"
<<<<<<< HEAD
        echo $nu.env.FOO
        with-env [FOO $nothing] {
            echo $nu.env.FOO
        }
        echo $nu.env.FOO
=======
        echo $env.FOO
        with-env [FOO $nothing] {
            echo $env.FOO
        }
        echo $env.FOO
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "11");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn with_env_shorthand_can_not_hide_variables() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
        let-env FOO = "1"
<<<<<<< HEAD
        echo $nu.env.FOO
        FOO=$nothing echo $nu.env.FOO
        echo $nu.env.FOO
=======
        echo $env.FOO
        FOO=$nothing echo $env.FOO
        echo $env.FOO
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "1$nothing1");
}
