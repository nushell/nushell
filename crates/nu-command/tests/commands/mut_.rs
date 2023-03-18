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

#[test]
fn mut_path_insert() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut y = {abc: 123}; $y.abc = 456; $y.abc
        "#
    ));

    assert_eq!(actual.out, "456");
}

#[test]
fn mut_path_insert_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut a = [0 1 2]; $a.3 = 3; $a | to nuon
        "#
    ));

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn mut_path_upsert() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut a = {b:[{c:1}]}; $a.b.0.d = 11; $a.b.0.d
        "#
    ));

    assert_eq!(actual.out, "11");
}

#[test]
fn mut_path_upsert_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut a = [[[3] 2] 1]; $a.0.0.1 = 0; $a.0.2 = 0; $a.2 = 0; $a | to nuon
        "#
    ));

    assert_eq!(actual.out, "[[[3, 0], 2, 0], 1, 0]");
}

#[test]
fn mut_path_operator_assign() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        mut a = {b:1}; $a.b += 3; $a.b -= 2; $a.b *= 10; $a.b /= 4; $a.b
        "#
    ));

    assert_eq!(actual.out, "5");
}
