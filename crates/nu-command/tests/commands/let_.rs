use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
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
fn let_typed_glob_expands_in_ls() {
    Playground::setup("let_glob_ls", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("a.toml"), EmptyFile("b.toml"), EmptyFile("c.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"let x: glob = "*.toml"; ls $x | length"#);
        assert_eq!(actual.out, "2");
    })
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

// ============================================================
// Tests for the three pipeline positions of `let`
// ============================================================

// --- LHS (beginning): `let x = expr` produces NO output ---

#[test]
fn let_lhs_produces_no_output() {
    // `let x = 5` should produce no visible output
    let actual = nu!("let x = 5");
    assert_eq!(actual.out, "");
}

#[test]
fn let_lhs_expression_produces_no_output() {
    // `let x = 10 + 100` should produce no visible output
    let actual = nu!("let x = 10 + 100");
    assert_eq!(actual.out, "");
}

#[test]
fn let_lhs_stores_value_correctly() {
    // Even though `let x = 10` produces no output, the variable is still set
    let actual = nu!("let x = 10; $x");
    assert_eq!(actual.out, "10");
}

#[test]
fn let_lhs_pipeline_expr_produces_no_output() {
    // `let x = "hello" | str length` should produce no output
    let actual = nu!(r#"let x = "hello world" | str length"#);
    assert_eq!(actual.out, "");
}

#[test]
fn let_lhs_invalid_pipeline() {
    let actual = nu!("let x = 5 | echo done");
    assert!(
        actual.err.contains("doesn't support") || actual.err.contains("invalid `let` keyword call")
    );
}

// --- Middle: `input | let var | next` passes value through ---

#[test]
fn let_mid_passes_value_through() {
    let actual = nu!("10 | let x | $x + 5");
    assert_eq!(actual.out, "15");
}

#[test]
fn let_mid_allows_in_variable() {
    let actual = nu!("10 | let x | $in + 5");
    assert_eq!(actual.out, "15");
}

#[test]
fn let_mid_passes_list_through() {
    let actual = nu!("[2 3 4] | let nums | first");
    assert_eq!(actual.out, "2");
}

#[test]
fn let_mid_passes_string_through() {
    let actual = nu!(r#""hello" | let msg | str length"#);
    assert_eq!(actual.out, "5");
}

#[test]
fn let_mid_pipeline_equivalence() {
    // `[2 3 4] | let nums | first` should be equivalent to assigning then piping
    let actual1 = nu!("[2 3 4] | let nums | first");
    let actual2 = nu!("[2 3 4] | let nums; $nums | first");
    assert_eq!(actual1.out, actual2.out);
}

// --- End: `input | let var` outputs the assigned value ---

#[test]
fn let_end_outputs_value() {
    // `5 | let x` should output the value 5
    let actual = nu!("5 | let x");
    assert_eq!(actual.out, "5");
}

#[test]
fn let_end_outputs_computed_value() {
    // Pipeline ending with let should output the computed result
    let actual = nu!(r#""hello" | str length | let n"#);
    assert_eq!(actual.out, "5");
}

#[test]
fn let_end_stores_value_correctly() {
    // The variable should also be usable afterward
    let actual = nu!("5 | let x; $x + 10");
    assert_eq!(actual.out, "15");
}

#[test]
#[ignore = "TODO: Need to detect at parse time that 'let x | echo done' is invalid"]
fn let_var_at_beginning_error() {
    let actual = nu!("let x | echo done");
    assert!(
        actual.err.contains("doesn't support") || actual.err.contains("invalid `let` keyword call")
    );
}
