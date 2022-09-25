use crate::tests::{fail_test, run_test, TestResult};

// TODO: Test the use/hide tests also as separate lines in REPL (i.e., with  merging the delta in between)
#[test]
fn hides_def() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_env() -> TestResult {
    fail_test(r#"let-env foo = "foo"; hide-env foo; $env.foo"#, "")
}

#[test]
fn hides_def_then_redefines() -> TestResult {
    // this one should fail because of predecl -- cannot have more defs with the same name in a
    // block
    fail_test(
        r#"def foo [] { "foo" }; hide foo; def foo [] { "bar" }; foo"#,
        "defined more than once",
    )
}

#[test]
fn hides_alias_then_redefines() -> TestResult {
    run_test(
        r#"alias foo = echo "foo"; hide foo; alias foo = echo "foo"; foo"#,
        "foo",
    )
}

#[test]
fn hides_env_then_redefines() -> TestResult {
    run_test(
        r#"let-env foo = "foo"; hide-env foo; let-env foo = "bar"; $env.foo"#,
        "bar",
    )
}

#[test]
fn hides_def_in_scope_1() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_in_scope_2() -> TestResult {
    run_test(
        r#"def foo [] { "foo" }; do { def foo [] { "bar" }; hide foo; foo }"#,
        "foo",
    )
}

#[test]
fn hides_def_in_scope_3() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { hide foo; def foo [] { "bar" }; hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_in_scope_4() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { def foo [] { "bar" }; hide foo; hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_in_scope_1() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; do { hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_in_scope_2() -> TestResult {
    run_test(
        r#"alias foo = echo "foo"; do { alias foo = echo "bar"; hide foo; foo }"#,
        "foo",
    )
}

#[test]
fn hides_alias_in_scope_3() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; do { hide foo; alias foo = echo "bar"; hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_in_scope_4() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; do { alias foo = echo "bar"; hide foo; hide foo; foo }"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_env_in_scope_1() -> TestResult {
    fail_test(
        r#"let-env foo = "foo"; do { hide-env foo; $env.foo }"#,
        "cannot find column",
    )
}

#[test]
fn hides_env_in_scope_2() -> TestResult {
    run_test(
        r#"let-env foo = "foo"; do { let-env foo = "bar"; hide-env foo; $env.foo }"#,
        "foo",
    )
}

#[test]
fn hides_env_in_scope_3() -> TestResult {
    fail_test(
        r#"let-env foo = "foo"; do { hide-env foo; let-env foo = "bar"; hide-env foo; $env.foo }"#,
        "",
    )
}

#[test]
fn hides_env_in_scope_4() -> TestResult {
    fail_test(
        r#"let-env foo = "foo"; do { let-env foo = "bar"; hide-env foo; hide-env foo; $env.foo }"#,
        "",
    )
}

#[test]
#[ignore]
fn hide_def_twice_not_allowed() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; hide foo; hide foo"#,
        "did not find",
    )
}

#[test]
#[ignore]
fn hide_alias_twice_not_allowed() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; hide foo; hide foo"#,
        "did not find",
    )
}

#[test]
fn hide_env_twice_not_allowed() -> TestResult {
    fail_test(r#"let-env foo = "foo"; hide-env foo; hide-env foo"#, "")
}

#[test]
fn hide_env_twice_allowed() -> TestResult {
    fail_test(
        r#"let-env foo = "foo"; hide-env foo; hide-env -i foo; $env.foo"#,
        "",
    )
}

#[test]
#[ignore = "Re-enable after virtualenv update"]
fn hides_def_runs_env_1() -> TestResult {
    run_test(
        r#"let-env foo = "bar"; def foo [] { "foo" }; hide foo; $env.foo"#,
        "bar",
    )
}

#[test]
#[ignore = "Re-enable after virtualenv update"]
fn hides_def_runs_env_2() -> TestResult {
    run_test(
        r#"def foo [] { "foo" }; let-env foo = "bar"; hide foo; $env.foo"#,
        "bar",
    )
}

#[test]
fn hides_alias_runs_def_1() -> TestResult {
    run_test(
        r#"def foo [] { "bar" }; alias foo = echo "foo"; hide foo; foo"#,
        "bar",
    )
}

#[test]
fn hides_alias_runs_def_2() -> TestResult {
    run_test(
        r#"alias foo = echo "foo"; def foo [] { "bar" }; hide foo; foo"#,
        "bar",
    )
}

#[test]
fn hides_alias_and_def() -> TestResult {
    fail_test(
        r#"alias foo = echo "foo"; def foo [] { "bar" }; hide foo; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_1() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam foo; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_2() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_3() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam [foo]; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_4() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam foo; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_5() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam *; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_6() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam *; hide spam *; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_def_import_then_reimports() -> TestResult {
    run_test(
        r#"module spam { export def foo [] { "foo" } }; use spam foo; hide foo; use spam foo; foo"#,
        "foo",
    )
}

#[test]
fn hides_alias_import_1() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam; hide spam foo; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_2() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam; hide spam; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_3() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam; hide spam [foo]; spam foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_4() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam foo; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_5() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam *; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_6() -> TestResult {
    fail_test(
        r#"module spam { export alias foo = "foo" }; use spam *; hide spam *; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_alias_import_then_reimports() -> TestResult {
    run_test(
        r#"module spam { export alias foo = "foo" }; use spam foo; hide foo; use spam foo; foo"#,
        "foo",
    )
}

#[test]
fn hides_env_import_1() -> TestResult {
    fail_test(
        r#"module spam { export-env { let-env foo = "foo" } }; use spam; hide-env foo; $env.foo"#,
        "",
    )
}

#[test]
#[ignore = "Re-enable after virtualenv update"]
fn hides_def_runs_env_import() -> TestResult {
    run_test(
        r#"module spam { export-env { let-env foo = "foo" }; export def foo [] { "bar" } }; use spam foo; hide foo; $env.foo"#,
        "foo",
    )
}

#[test]
fn hides_def_and_env_import_1() -> TestResult {
    fail_test(
        r#"module spam { export-env { let-env foo = "foo" }; export def foo [] { "bar" } }; use spam foo; hide foo; hide-env foo; $env.foo"#,
        "",
    )
}

#[test]
fn use_def_import_after_hide() -> TestResult {
    run_test(
        r#"module spam { export def foo [] { "foo" } }; use spam foo; hide foo; use spam foo; foo"#,
        "foo",
    )
}

#[test]
fn use_env_import_after_hide() -> TestResult {
    run_test(
        r#"module spam { export-env { let-env foo = "foo" } }; use spam; hide-env foo; use spam; $env.foo"#,
        "foo",
    )
}

#[test]
fn hide_shadowed_decl() -> TestResult {
    run_test(
        r#"module spam { export def foo [] { "bar" } }; def foo [] { "foo" }; do { use spam foo; hide foo; foo }"#,
        "foo",
    )
}

#[test]
fn hide_shadowed_env() -> TestResult {
    run_test(
        r#"module spam { export-env { let-env foo = "bar" } }; let-env foo = "foo"; do { use spam; hide-env foo; $env.foo }"#,
        "foo",
    )
}

#[test]
fn hides_all_decls_within_scope() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "bar" } }; def foo [] { "foo" }; use spam foo; hide foo; foo"#,
        "", // we just care if it errors
    )
}

#[test]
fn hides_all_envs_within_scope() -> TestResult {
    fail_test(
        r#"module spam { export-env { let-env foo = "bar" } }; let-env foo = "foo"; use spam; hide-env foo; $env.foo"#,
        "",
    )
}
