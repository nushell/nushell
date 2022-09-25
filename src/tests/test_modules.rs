use crate::tests::{fail_test, run_test, TestResult};

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
        r#"module foo { export-env { let-env a = '1' } }; use foo; $env.a"#,
        "1",
    )
}

#[test]
fn module_env_imports_2() -> TestResult {
    run_test(
        r#"module foo { export-env { let-env a = '1'; let-env b = '2' } }; use foo; $env.b"#,
        "2",
    )
}

#[test]
fn module_env_imports_3() -> TestResult {
    run_test(
        r#"module foo { export-env { let-env a = '1' }; export-env { let-env b = '2' }; export-env {let-env c = '3'} }; use foo; $env.c"#,
        "3",
    )
}

#[test]
fn module_def_and_env_imports_1() -> TestResult {
    run_test(
        r#"module spam { export-env { let-env foo = "foo" }; export def foo [] { "bar" } }; use spam; $env.foo"#,
        "foo",
    )
}

#[test]
fn module_def_and_env_imports_2() -> TestResult {
    run_test(
        r#"module spam { export-env { let-env foo = "foo" }; export def foo [] { "bar" } }; use spam foo; foo"#,
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
        r#"module foo { def b [] { "2" }; export-env { let-env a = b }  }; use foo; $env.a"#,
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
