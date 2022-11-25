use nu_test_support::{nu, pipeline};

#[test]
fn mut_variable() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut x = 3; $x = $x + 1; $x
        "#
    ));

    assert_eq!(actual.out, "4");
}

#[test]
fn mut_variable_in_loop() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut x = 1; for i in 1..10 { $x = $x + $i}; $x
        "#
    ));

    assert_eq!(actual.out, "56");
}

#[test]
fn capture_of_mutable_var() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut x = 123; {|| $x }
        "#
    ));

    assert!(actual.err.contains("capture of mutable variable"));
}

#[test]
fn mut_a_field() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = {abc: 123}; $y.abc = 456; $y.abc
        "#
    ));

    assert_eq!(actual.out, "456");
}

#[test]
fn mut_add_assign() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = 3; $y += 2; $y
        "#
    ));

    assert_eq!(actual.out, "5");
}

#[test]
fn mut_minus_assign() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = 3; $y -= 2; $y
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn mut_multiply_assign() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = 3; $y *= 2; $y
        "#
    ));

    assert_eq!(actual.out, "6");
}

#[test]
fn mut_divide_assign() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = 8; $y /= 2; $y
        "#
    ));

    assert_eq!(actual.out, "4");
}
