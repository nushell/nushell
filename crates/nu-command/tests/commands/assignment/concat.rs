use nu_test_support::nu;

#[test]
fn concat_assign_list_int() {
    let actual = nu!(r#"
        mut a = [1 2];
        $a ++= [3 4];
        $a == [1 2 3 4]
    "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn concat_assign_list_string() {
    let actual = nu!(r#"
        mut a = [a b];
        $a ++= [c d];
        $a == [a b c d]
    "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn concat_assign_any() {
    let actual = nu!(r#"
        mut a = [1 2 a];
        $a ++= [b 3];
        $a == [1 2 a b 3]
    "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn concat_assign_both_empty() {
    let actual = nu!(r#"
        mut a = [];
        $a ++= [];
        $a == []
    "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn concat_assign_string() {
    let actual = nu!(r#"
        mut a = 'hello';
        $a ++= ' world';
        $a == 'hello world'
    "#);

    assert_eq!(actual.out, "true")
}

#[test]
fn concat_assign_type_mismatch() {
    let actual = nu!(r#"
        mut a = [];
        $a ++= 'str'
    "#);

    assert!(actual.err.contains("nu::parser::unsupported_operation"));
}

#[test]
fn concat_assign_runtime_type_mismatch() {
    let actual = nu!(r#"
        mut a = [];
        $a ++= if true { 'str' }
    "#);

    assert!(actual.err.contains("nu::shell::type_mismatch"));
}
