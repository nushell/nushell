use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
use rstest::rstest;

#[rstest]
#[case("let in = 3")]
#[case("let in: int = 3")]
fn let_name_builtin_var(#[case] assignment: &str) -> Result {
    let err = test().run(assignment).expect_parse_error()?;
    assert_contains("`in` used as variable name", err.to_string());
    Ok(())
}

#[test]
fn let_doesnt_mutate() -> Result {
    let err = test().run("let i = 3; $i = 4").expect_parse_error()?;
    assert_contains("immutable", err.to_string());
    Ok(())
}

#[test]
fn let_takes_pipeline() -> Result {
    test()
        .run(r#"let x = "hello world" | str length; $x"#)
        .expect_value_eq(11)
}

#[test]
fn let_takes_pipeline_with_declared_type() -> Result {
    test()
        .run(r#"let x: list<string> = [] | append "hello world"; $x.0"#)
        .expect_value_eq("hello world")
}

#[test]
fn let_pipeline_allows_in() -> Result {
    test()
        .run(r#"def foo [] { let x = $in | str length; $x + 10 }; "hello world" | foo"#)
        .expect_value_eq(21)
}

#[test]
fn mut_takes_pipeline() -> Result {
    test()
        .run(r#"mut x = "hello world" | str length; $x"#)
        .expect_value_eq(11)
}

#[test]
fn mut_takes_pipeline_with_declared_type() -> Result {
    test()
        .run(r#"mut x: list<string> = [] | append "hello world"; $x.0"#)
        .expect_value_eq("hello world")
}

#[test]
fn mut_pipeline_allows_in() -> Result {
    test()
        .run(r#"def foo [] { mut x = $in | str length; $x + 10 }; "hello world" | foo"#)
        .expect_value_eq(21)
}

#[test]
fn let_pipeline_redirects_internals() -> Result {
    test()
        .run("let x = echo 'bar'; $x | str length")
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn let_pipeline_redirects_externals() -> Result {
    test()
        .run("let x = cococo 'bar'; $x | str length")
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn let_err_pipeline_redirects_externals() -> Result {
    test()
        .run(r#"let x = with-env { FOO: "foo" } { echo_env_stderr FOO e>| str length }; $x"#)
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn let_outerr_pipeline_redirects_externals() -> Result {
    test()
        .run(r#"let x = with-env { FOO: "foo" } { echo_env_stderr FOO o+e>| str length }; $x"#)
        .expect_value_eq(3)
}

#[ignore]
#[test]
#[deps(TESTBIN_FAIL)]
fn let_with_external_failed() -> Result {
    // FIXME: this test hasn't run successfully for a long time. We should
    // bring it back to life at some point.
    test()
        .run("let x = fail; echo fail")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
fn let_glob_type() -> Result {
    test()
        .run("let x: glob = 'aa'; $x | describe")
        .expect_value_eq("glob")
}

#[test]
fn let_typed_glob_expands_in_ls() -> Result {
    Playground::setup("let_glob_ls", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("a.toml"), EmptyFile("b.toml"), EmptyFile("c.txt")]);

        test()
            .cwd(dirs.test())
            .run(r#"let x: glob = "*.toml"; ls $x | length"#)
            .expect_value_eq(2)
    })
}

#[test]
fn let_raw_string() -> Result {
    test()
        .run(r#"let x = r#'abcde""fghi"''''jkl'#; $x"#)
        .expect_value_eq(r#"abcde""fghi"''''jkl"#)?;
    test()
        .run(r#"let x = r##'abcde""fghi"''''#jkl'##; $x"#)
        .expect_value_eq(r#"abcde""fghi"''''#jkl"#)?;
    test()
        .run(r#"let x = r###'abcde""fghi"'''##'#jkl'###; $x"#)
        .expect_value_eq(r#"abcde""fghi"'''##'#jkl"#)?;
    test().run("let x = r#'abc'#; $x").expect_value_eq("abc")
}

#[test]
fn let_malformed_type() -> Result {
    let err = test().run("let foo: )a").expect_parse_error()?;
    assert_contains("Unbalanced delimiter", err.to_string());

    let err = test().run("let foo: }a").expect_parse_error()?;
    assert_contains("Unbalanced delimiter", err.to_string());

    let err = test().run("mut : , a").expect_parse_error()?;
    assert_contains("Unknown type", err.to_string());
    Ok(())
}

// ============================================================
// Tests for the three pipeline positions of `let`
// ============================================================

// --- LHS (beginning): `let x = expr` produces NO output ---

#[test]
fn let_lhs_produces_no_output() -> Result {
    test().run("let x = 5").expect_value_eq(())
}

#[test]
fn let_lhs_expression_produces_no_output() -> Result {
    test().run("let x = 10 + 100").expect_value_eq(())
}

#[test]
fn let_lhs_stores_value_correctly() -> Result {
    test().run("let x = 10; $x").expect_value_eq(10)
}

#[test]
fn let_lhs_pipeline_expr_produces_no_output() -> Result {
    test()
        .run(r#"let x = "hello world" | str length"#)
        .expect_value_eq(())
}

#[test]
fn let_lhs_invalid_pipeline() -> Result {
    let _ = test().run("let x = 5 | echo done").expect_parse_error()?;
    Ok(())
}

// --- Middle: `input | let var | next` passes value through ---

#[test]
fn let_mid_passes_value_through() -> Result {
    test().run("10 | let x | $x + 5").expect_value_eq(15)
}

#[test]
fn let_mid_allows_in_variable() -> Result {
    test().run("10 | let x | $in + 5").expect_value_eq(15)
}

#[test]
fn let_mid_passes_list_through() -> Result {
    test().run("[2 3 4] | let nums | first").expect_value_eq(2)
}

#[test]
fn let_mid_passes_string_through() -> Result {
    test()
        .run(r#""hello" | let msg | str length"#)
        .expect_value_eq(5)
}

#[test]
fn let_mid_pipeline_equivalence() -> Result {
    let actual1: i64 = test().run("[2 3 4] | let nums | first")?;
    let actual2: i64 = test().run("[2 3 4] | let nums; $nums | first")?;
    assert_eq!(actual1, actual2);
    Ok(())
}

// --- End: `input | let var` outputs the assigned value ---

#[test]
fn let_end_outputs_value() -> Result {
    test().run("5 | let x").expect_value_eq(5)
}

#[test]
fn let_end_outputs_computed_value() -> Result {
    test()
        .run(r#""hello" | str length | let n"#)
        .expect_value_eq(5)
}

#[test]
fn let_end_stores_value_correctly() -> Result {
    test().run("5 | let x; $x + 10").expect_value_eq(15)
}

#[test]
#[ignore = "TODO: Need to detect at parse time that 'let x | echo done' is invalid"]
fn let_var_at_beginning_error() -> Result {
    let err = test().run("let x | echo done").expect_error()?;
    let msg = err.to_string();
    assert!(msg.contains("doesn't support") || msg.contains("invalid `let` keyword call"));
    Ok(())
}
