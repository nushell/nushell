use nu_test_support::nu;

#[test]
fn unlet_basic() {
    let actual = nu!("let x = 42; unlet $x; $x");

    assert!(actual.err.contains("Variable not found"));
}

#[test]
fn unlet_builtin_nu() {
    let actual = nu!("unlet $nu");

    assert!(actual.err.contains("cannot be deleted"));
}

#[test]
fn unlet_builtin_env() {
    let actual = nu!("unlet $env");

    assert!(actual.err.contains("cannot be deleted"));
}

#[test]
fn unlet_not_variable() {
    let actual = nu!("unlet 42");

    assert!(
        actual
            .err
            .contains("Argument must be a variable reference like $x")
    );
}

#[test]
fn unlet_wrong_number_args() {
    let actual = nu!("unlet");

    assert!(actual.err.contains("Missing required positional argument"));
}

#[test]
fn unlet_multiple_args() {
    let actual = nu!("let x = 1; let y = 2; unlet $x $y");

    assert!(actual.err.contains("Extra positional argument"));
}
