use nu_test_support::{nu, pipeline};

#[test]
fn let_name_builtin_var() {
    let actual = nu!("let in = 3");

    assert!(actual
        .err
        .contains("'in' is the name of a builtin Nushell variable"));
}

#[test]
fn let_doesnt_mutate() {
    let actual = nu!("let i = 3; $i = 4");

    assert!(actual.err.contains("immutable"));
}

#[test]
fn let_takes_pipeline() {
    let actual = nu!(r#"let x = "hello world" | str length; print $x"#);

    assert_eq!(actual.out, "11");
}

#[test]
fn let_pipeline_allows_in() {
    let actual =
        nu!(r#"def foo [] { let x = $in | str length; print ($x + 10) }; "hello world" | foo"#);

    assert_eq!(actual.out, "21");
}

#[test]
fn let_with_no_spaces_1() {
    let actual = nu!("let x=4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_2() {
    let actual = nu!("let x =4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_3() {
    let actual = nu!("let x= 4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_4() {
    let actual = nu!("let x: int= 4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_5() {
    let actual = nu!("let x:int= 4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_6() {
    let actual = nu!("let x:int=4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_7() {
    let actual = nu!("let x : int = 4; $x");

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_if() {
    let actual = nu!("let x=if true { 1 } else { 0 }; $x");

    assert_eq!(actual.out, "1");
}

#[test]
fn let_with_match() {
    let actual = nu!("let x =match 4 { 5 => 1, _ => 0 }; $x");

    assert_eq!(actual.out, "0");
}

#[test]
fn let_with_complex_type() {
    let actual = nu!("let x: record<name: string> = { name: 'nushell' }; $x.name");

    assert_eq!(actual.out, "nushell");
}

#[ignore]
#[test]
fn let_with_external_failed() {
    // FIXME: this test hasn't run successfully for a long time. We should
    // bring it back to life at some point.
    let actual = nu!(r#"let x = nu --testbin outcome_err "aa"; echo fail"#);

    assert!(!actual.out.contains("fail"));
}
