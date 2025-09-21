use nu_test_support::nu;
use rstest::rstest;

#[rstest]
#[case("let in = 3")]
#[case("let in: int = 3")]
fn let_name_builtin_var(#[case] assignment: &str) {
    assert!(
        nu!(assignment)
            .err
            .contains("'in' is the name of a builtin Nushell variable")
    );
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
fn let_takes_pipeline_with_declared_type() {
    let actual = nu!(r#"let x: list<string> = [] | append "hello world"; print $x.0"#);

    assert_eq!(actual.out, "hello world");
}

#[test]
fn let_pipeline_allows_in() {
    let actual =
        nu!(r#"def foo [] { let x = $in | str length; print ($x + 10) }; "hello world" | foo"#);

    assert_eq!(actual.out, "21");
}

#[test]
fn mut_takes_pipeline() {
    let actual = nu!(r#"mut x = "hello world" | str length; print $x"#);

    assert_eq!(actual.out, "11");
}

#[test]
fn mut_takes_pipeline_with_declared_type() {
    let actual = nu!(r#"mut x: list<string> = [] | append "hello world"; print $x.0"#);

    assert_eq!(actual.out, "hello world");
}

#[test]
fn mut_pipeline_allows_in() {
    let actual =
        nu!(r#"def foo [] { mut x = $in | str length; print ($x + 10) }; "hello world" | foo"#);

    assert_eq!(actual.out, "21");
}

#[test]
fn let_pipeline_redirects_internals() {
    let actual = nu!(r#"let x = echo 'bar'; $x | str length"#);

    assert_eq!(actual.out, "3");
}

#[test]
fn let_pipeline_redirects_externals() {
    let actual = nu!(r#"let x = nu --testbin cococo 'bar'; $x | str length"#);

    assert_eq!(actual.out, "3");
}

#[test]
fn let_err_pipeline_redirects_externals() {
    let actual = nu!(
        r#"let x = with-env { FOO: "foo" } {nu --testbin echo_env_stderr FOO e>| str length}; $x"#
    );
    assert_eq!(actual.out, "3");
}

#[test]
fn let_outerr_pipeline_redirects_externals() {
    let actual = nu!(
        r#"let x = with-env { FOO: "foo" } {nu --testbin echo_env_stderr FOO o+e>| str length}; $x"#
    );
    assert_eq!(actual.out, "3");
}

#[ignore]
#[test]
fn let_with_external_failed() {
    // FIXME: this test hasn't run successfully for a long time. We should
    // bring it back to life at some point.
    let actual = nu!(r#"let x = nu --testbin outcome_err "aa"; echo fail"#);

    assert!(!actual.out.contains("fail"));
}

#[test]
fn let_glob_type() {
    let actual = nu!("let x: glob = 'aa'; $x | describe");
    assert_eq!(actual.out, "glob");
}

#[test]
fn let_raw_string() {
    let actual = nu!(r#"let x = r#'abcde""fghi"''''jkl'#; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"''''jkl"#);

    let actual = nu!(r#"let x = r##'abcde""fghi"''''#jkl'##; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"''''#jkl"#);

    let actual = nu!(r#"let x = r###'abcde""fghi"'''##'#jkl'###; $x"#);
    assert_eq!(actual.out, r#"abcde""fghi"'''##'#jkl"#);

    let actual = nu!(r#"let x = r#'abc'#; $x"#);
    assert_eq!(actual.out, "abc");
}

#[test]
fn let_malformed_type() {
    let actual = nu!("let foo: )a");
    assert!(actual.err.contains("unbalanced ( and )"));

    let actual = nu!("let foo: }a");
    assert!(actual.err.contains("unbalanced { and }"));

    let actual = nu!("mut : , a");
    assert!(actual.err.contains("unknown type"));
}
