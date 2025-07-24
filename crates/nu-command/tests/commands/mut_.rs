use nu_test_support::nu;
use rstest::rstest;

#[test]
fn mut_variable() {
    let actual = nu!("mut x = 3; $x = $x + 1; $x");

    assert_eq!(actual.out, "4");
}

#[rstest]
#[case("mut in = 3")]
#[case("mut in: int = 3")]
fn mut_name_builtin_var(#[case] assignment: &str) {
    assert!(
        nu!(assignment)
            .err
            .contains("'in' is the name of a builtin Nushell variable")
    );
}

#[test]
fn mut_name_builtin_var_with_dollar() {
    let actual = nu!("mut $env = 3");

    assert!(
        actual
            .err
            .contains("'env' is the name of a builtin Nushell variable")
    )
}

#[test]
fn mut_variable_in_loop() {
    let actual = nu!("mut x = 1; for i in 1..10 { $x = $x + $i}; $x");

    assert_eq!(actual.out, "56");
}

#[test]
fn capture_of_mutable_var() {
    let actual = nu!("mut x = 123; {|| $x }");

    assert!(actual.err.contains("capture of mutable variable"));
}

#[test]
fn mut_add_assign() {
    let actual = nu!("mut y = 3; $y += 2; $y");

    assert_eq!(actual.out, "5");
}

#[test]
fn mut_minus_assign() {
    let actual = nu!("mut y = 3; $y -= 2; $y");

    assert_eq!(actual.out, "1");
}

#[test]
fn mut_multiply_assign() {
    let actual = nu!("mut y = 3; $y *= 2; $y");

    assert_eq!(actual.out, "6");
}

#[test]
fn mut_divide_assign() {
    let actual = nu!("mut y: number = 8; $y /= 2; $y");

    assert_eq!(actual.out, "4.0");
}

#[test]
fn mut_divide_assign_should_error() {
    let actual = nu!("mut y = 8; $y /= 2; $y");

    assert!(actual.err.contains("parser::operator_incompatible_types"));
}

#[test]
fn mut_subtract_assign_should_error() {
    let actual = nu!("mut x = (date now); $x -= 2019-05-10");

    assert!(actual.err.contains("parser::operator_incompatible_types"));
}

#[test]
fn mut_assign_number() {
    let actual = nu!("mut x: number = 1; $x = 2.0; $x");

    assert_eq!(actual.out, "2.0");
}

#[test]
fn mut_assign_glob() {
    let actual = nu!(r#"mut x: glob = ""; $x = "meow"; $x"#);

    assert_eq!(actual.out, "meow");
}

#[test]
fn mut_path_insert() {
    let actual = nu!("mut y = {abc: 123}; $y.abc = 456; $y.abc");

    assert_eq!(actual.out, "456");
}

#[test]
fn mut_path_insert_list() {
    let actual = nu!("mut a = [0 1 2]; $a.3 = 3; $a | to nuon");

    assert_eq!(actual.out, "[0, 1, 2, 3]");
}

#[test]
fn mut_path_upsert() {
    let actual = nu!("mut a = {b:[{c:1}]}; $a.b.0.d = 11; $a.b.0.d");

    assert_eq!(actual.out, "11");
}

#[test]
fn mut_path_upsert_list() {
    let actual = nu!("mut a = [[[3] 2] 1]; $a.0.0.1 = 0; $a.0.2 = 0; $a.2 = 0; $a | to nuon");

    assert_eq!(actual.out, "[[[3, 0], 2, 0], 1, 0]");
}

#[test]
fn mut_path_operator_assign() {
    let actual = nu!("mut a = {b:1}; $a.b += 3; $a.b -= 2; $a.b *= 10; $a.b /= 4; $a.b");

    assert_eq!(actual.out, "5.0");
}

#[test]
fn mut_records_update_properly() {
    let actual = nu!("mut a = {}; $a.b.c = 100; $a.b.c");
    assert_eq!(actual.out, "100");
}

#[test]
fn mut_value_with_if() {
    let actual = nu!("mut a = 3; $a = if 3 == 3 { 10 }; echo $a");
    assert_eq!(actual.out, "10");
}

#[test]
fn mut_value_with_match() {
    let actual = nu!("mut a = 3; $a = match 3 { 1 => { 'yes!' }, _ => { 'no!' } }; echo $a");
    assert_eq!(actual.out, "no!");
}

#[test]
fn mut_glob_type() {
    let actual = nu!("mut x: glob = 'aa'; $x | describe");
    assert_eq!(actual.out, "glob");
}

#[test]
fn mut_raw_string() {
    let actual = nu!(r#"mut x = r#'abcde""fghi"''''jkl'#; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"''''jkl"#);

    let actual = nu!(r#"mut x = r##'abcde""fghi"''''#jkl'##; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"''''#jkl"#);

    let actual = nu!(r#"mut x = r###'abcde""fghi"'''##'#jkl'###; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"'''##'#jkl"#);

    let actual = nu!(r#"mut x = r#'abc'#; $x"#);
    assert_eq!(actual.out, "abc");
}

#[test]
fn def_should_not_mutate_mut() {
    let actual = nu!("mut a = 3; def foo [] { $a = 4}");
    assert!(actual.err.contains("capture of mutable variable"));
    assert!(!actual.status.success())
}

#[test]
fn assign_to_non_mut_variable_raises_parse_error() {
    let actual = nu!("let x = 3; $x = 4");
    assert!(
        actual
            .err
            .contains("parser::assignment_requires_mutable_variable")
    );

    let actual = nu!("mut x = 3; x = 5");
    assert!(actual.err.contains("parser::assignment_requires_variable"));
}
