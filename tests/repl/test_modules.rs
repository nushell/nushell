use crate::repl::tests::{fail_test, run_test, TestResult};
use rstest::rstest;

#[test]
fn module_def_imports_1() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { 2 } }; use foo; foo a"#,
        "1",
    )
}

#[test]
fn module_def_imports_2() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { 2 } }; use foo a; a"#,
        "1",
    )
}

#[test]
fn module_def_imports_3() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; export def b [] { 2 } }; use foo *; b"#,
        "2",
    )
}

#[test]
fn module_def_imports_4() -> TestResult {
    fail_test(
        r#"module foo { export def a [] { 1 }; export def b [] { 2 } }; use foo c"#,
        "not find import",
    )
}

#[test]
fn module_def_imports_5() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { '2' }; export def c [] { '3' } }; use foo [a, c]; c"#,
        "3",
    )
}

#[test]
fn module_env_imports_1() -> TestResult {
    run_test(
        r#"module foo { export-env { $env.a = '1' } }; use foo; $env.a"#,
        "1",
    )
}

#[test]
fn module_env_imports_2() -> TestResult {
    run_test(
        r#"module foo { export-env { $env.a = '1'; $env.b = '2' } }; use foo; $env.b"#,
        "2",
    )
}

#[test]
fn module_env_imports_3() -> TestResult {
    run_test(
        r#"module foo { export-env { $env.a = '1' }; export-env { $env.b = '2' }; export-env {$env.c = '3'} }; use foo; $env.c"#,
        "3",
    )
}

#[test]
fn module_def_and_env_imports_1() -> TestResult {
    run_test(
        r#"module spam { export-env { $env.foo = "foo" }; export def foo [] { "bar" } }; use spam; $env.foo"#,
        "foo",
    )
}

#[test]
fn module_def_and_env_imports_2() -> TestResult {
    run_test(
        r#"module spam { export-env { $env.foo = "foo" }; export def foo [] { "bar" } }; use spam foo; foo"#,
        "bar",
    )
}

#[test]
fn module_def_import_uses_internal_command() -> TestResult {
    run_test(
        r#"module foo { def b [] { 2 }; export def a [] { b }  }; use foo; foo a"#,
        "2",
    )
}

#[test]
fn module_env_import_uses_internal_command() -> TestResult {
    run_test(
        r#"module foo { def b [] { "2" }; export-env { $env.a = (b) }  }; use foo; $env.a"#,
        "2",
    )
}

#[test]
fn multi_word_imports() -> TestResult {
    run_test(
        r#"module spam { export def "foo bar" [] { 10 } }; use spam "foo bar"; foo bar"#,
        "10",
    )
}

#[test]
fn export_alias() -> TestResult {
    run_test(
        r#"module foo { export alias hi = echo hello }; use foo hi; hi"#,
        "hello",
    )
}

#[test]
fn export_consts() -> TestResult {
    run_test(
        r#"module spam { export const b = 3; }; use spam b; $b"#,
        "3",
    )?;
    run_test(
        r#"module spam { export const b: int = 3; }; use spam b; $b"#,
        "3",
    )
}

#[test]
fn dont_export_module_name_as_a_variable() -> TestResult {
    fail_test(r#"module spam { }; use spam; $spam"#, "variable not found")
}

#[test]
fn func_use_consts() -> TestResult {
    run_test(
        r#"module spam { const b = 3; export def c [] { $b } }; use spam; spam c"#,
        "3",
    )
}

#[test]
fn export_module_which_defined_const() -> TestResult {
    run_test(
        r#"module spam { export const b = 3; export const c = 4 }; use spam; $spam.b + $spam.c"#,
        "7",
    )
}

#[rstest]
#[case("spam-mod")]
#[case("spam/mod")]
#[case("spam=mod")]
fn export_module_with_normalized_var_name(#[case] name: &str) -> TestResult {
    let def = format!(
        "module {name} {{ export const b = 3; export module {name}2 {{ export const c = 4 }}  }}"
    );
    run_test(&format!("{def}; use {name}; $spam_mod.b"), "3")?;
    run_test(&format!("{def}; use {name} *; $spam_mod2.c"), "4")
}

#[rstest]
#[case("spam-mod")]
#[case("spam/mod")]
fn use_module_with_invalid_var_name(#[case] name: &str) -> TestResult {
    fail_test(
        &format!("module {name} {{ export const b = 3 }}; use {name}; ${name}"),
        "expected valid variable name. Did you mean '$spam_mod'",
    )
}

#[test]
fn cannot_export_private_const() -> TestResult {
    fail_test(
        r#"module spam { const b = 3; export const c = 4 }; use spam; $spam.b + $spam.c"#,
        "cannot find column 'b'",
    )
}

#[test]
fn test_lexical_binding() -> TestResult {
    run_test(
        r#"module spam { const b = 3; export def c [] { $b } }; use spam c; const b = 4; c"#,
        "3",
    )?;
    run_test(
        r#"const b = 4; module spam { const b = 3; export def c [] { $b } }; use spam; spam c"#,
        "3",
    )
}
