use nu_test_support::nu;

#[test]
fn delvar_basic() {
    let actual = nu!("let x = 42; delvar $x; $x");

    assert!(actual.err.contains("Variable not found"));
}

#[test]
fn delvar_builtin_nu() {
    let actual = nu!("delvar $nu");

    assert!(actual.err.contains("cannot be deleted"));
}

#[test]
fn delvar_builtin_env() {
    let actual = nu!("delvar $env");

    assert!(actual.err.contains("cannot be deleted"));
}

#[test]
fn delvar_not_variable() {
    let actual = nu!("delvar 42");

    assert!(
        actual
            .err
            .contains("Argument must be a variable reference like $x")
    );
}

#[test]
fn delvar_wrong_number_args() {
    let actual = nu!("delvar");

    assert!(actual.err.contains("Missing required positional argument"));
}

#[test]
fn delvar_multiple_args() {
    let actual = nu!("let x = 1; let y = 2; delvar $x $y");

    assert!(actual.err.contains("Extra positional argument"));
}
