use crate::repl::tests::{fail_test, run_test, TestResult};

// TODO: Test the use/hide tests also as separate lines in REPL (i.e., with  merging the delta in between)
#[test]
fn hides_def() -> TestResult {
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias() -> TestResult {
    fail_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_env() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; hide-env myfoosymbol; $env.myfoosymbol"#,
        "",
    )
}

#[test]
fn hides_def_then_redefines() -> TestResult {
    // this one should fail because of predecl -- cannot have more defs with the same name in a
    // block
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; hide myfoosymbol; def myfoosymbol [] { "bar" }; myfoosymbol"#,
        "defined more than once",
    )
}

#[ignore = "TODO: We'd need to make predecls work with hiding as well"]
#[test]
fn hides_alias_then_redefines() -> TestResult {
    run_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; hide myfoosymbol; alias myfoosymbol = echo "myfoosymbol"; myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_env_then_redefines() -> TestResult {
    run_test(
        r#"$env.myfoosymbol = "myfoosymbol"; hide-env myfoosymbol; $env.myfoosymbol = "bar"; $env.myfoosymbol"#,
        "bar",
    )
}

#[test]
fn hides_def_in_scope_1() -> TestResult {
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; do { hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_def_in_scope_2() -> TestResult {
    run_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; do { def myfoosymbol [] { "bar" }; hide myfoosymbol; myfoosymbol }"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_def_in_scope_3() -> TestResult {
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; do { hide myfoosymbol; def myfoosymbol [] { "bar" }; hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_def_in_scope_4() -> TestResult {
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; do { def myfoosymbol [] { "bar" }; hide myfoosymbol; hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_alias_in_scope_1() -> TestResult {
    fail_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; do { hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_alias_in_scope_2() -> TestResult {
    run_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; do { alias myfoosymbol = echo "bar"; hide myfoosymbol; myfoosymbol }"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_alias_in_scope_3() -> TestResult {
    fail_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; do { hide myfoosymbol; alias myfoosymbol = echo "bar"; hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_alias_in_scope_4() -> TestResult {
    fail_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; do { alias myfoosymbol = echo "bar"; hide myfoosymbol; hide myfoosymbol; myfoosymbol }"#,
        "external_command",
    )
}

#[test]
fn hides_env_in_scope_1() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; do { hide-env myfoosymbol; $env.myfoosymbol }"#,
        "not_found",
    )
}

#[test]
fn hides_env_in_scope_2() -> TestResult {
    run_test(
        r#"$env.myfoosymbol = "myfoosymbol"; do { $env.myfoosymbol = "bar"; hide-env myfoosymbol; $env.myfoosymbol }"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_env_in_scope_3() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; do { hide-env myfoosymbol; $env.myfoosymbol = "bar"; hide-env myfoosymbol; $env.myfoosymbol }"#,
        "",
    )
}

#[test]
fn hides_env_in_scope_4() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; do { $env.myfoosymbol = "bar"; hide-env myfoosymbol; hide-env myfoosymbol; $env.myfoosymbol }"#,
        "",
    )
}

#[test]
#[ignore]
fn hide_def_twice_not_allowed() -> TestResult {
    fail_test(
        r#"def myfoosymbol [] { "myfoosymbol" }; hide myfoosymbol; hide myfoosymbol"#,
        "did not find",
    )
}

#[test]
#[ignore]
fn hide_alias_twice_not_allowed() -> TestResult {
    fail_test(
        r#"alias myfoosymbol = echo "myfoosymbol"; hide myfoosymbol; hide myfoosymbol"#,
        "did not find",
    )
}

#[test]
fn hide_env_twice_not_allowed() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; hide-env myfoosymbol; hide-env myfoosymbol"#,
        "",
    )
}

#[test]
fn hide_env_twice_allowed() -> TestResult {
    fail_test(
        r#"$env.myfoosymbol = "myfoosymbol"; hide-env myfoosymbol; hide-env -i myfoosymbol; $env.myfoosymbol"#,
        "",
    )
}

#[test]
fn hides_def_runs_env() -> TestResult {
    run_test(
        r#"$env.myfoosymbol = "bar"; def myfoosymbol [] { "myfoosymbol" }; hide myfoosymbol; $env.myfoosymbol"#,
        "bar",
    )
}

#[test]
fn hides_def_import_1() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule; hide myspammodule myfoosymbol; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_2() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule; hide myspammodule; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_3() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule; hide myspammodule [myfoosymbol]; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_4() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule myfoosymbol; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_5() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule *; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_6() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule *; hide myspammodule *; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_def_import_then_reimports() -> TestResult {
    run_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule myfoosymbol; hide myfoosymbol; use myspammodule myfoosymbol; myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_alias_import_1() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule; hide myspammodule myfoosymbol; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_2() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule; hide myspammodule; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_3() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule; hide myspammodule [myfoosymbol]; myspammodule myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_4() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule myfoosymbol; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_5() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule *; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_6() -> TestResult {
    fail_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule *; hide myspammodule *; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_alias_import_then_reimports() -> TestResult {
    run_test(
        r#"module myspammodule { export alias myfoosymbol = echo "myfoosymbol" }; use myspammodule myfoosymbol; hide myfoosymbol; use myspammodule myfoosymbol; myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_env_import_1() -> TestResult {
    fail_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "myfoosymbol" } }; use myspammodule; hide-env myfoosymbol; $env.myfoosymbol"#,
        "",
    )
}

#[test]
fn hides_def_runs_env_import() -> TestResult {
    run_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "myfoosymbol" }; export def myfoosymbol [] { "bar" } }; use myspammodule myfoosymbol; hide myfoosymbol; $env.myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_def_and_env_import_1() -> TestResult {
    fail_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "myfoosymbol" }; export def myfoosymbol [] { "bar" } }; use myspammodule myfoosymbol; hide myfoosymbol; hide-env myfoosymbol; $env.myfoosymbol"#,
        "",
    )
}

#[test]
fn use_def_import_after_hide() -> TestResult {
    run_test(
        r#"module myspammodule { export def myfoosymbol [] { "myfoosymbol" } }; use myspammodule myfoosymbol; hide myfoosymbol; use myspammodule myfoosymbol; myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn use_env_import_after_hide() -> TestResult {
    run_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "myfoosymbol" } }; use myspammodule; hide-env myfoosymbol; use myspammodule; $env.myfoosymbol"#,
        "myfoosymbol",
    )
}

#[test]
fn hide_shadowed_decl() -> TestResult {
    run_test(
        r#"module myspammodule { export def myfoosymbol [] { "bar" } }; def myfoosymbol [] { "myfoosymbol" }; do { use myspammodule myfoosymbol; hide myfoosymbol; myfoosymbol }"#,
        "myfoosymbol",
    )
}

#[test]
fn hide_shadowed_env() -> TestResult {
    run_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "bar" } }; $env.myfoosymbol = "myfoosymbol"; do { use myspammodule; hide-env myfoosymbol; $env.myfoosymbol }"#,
        "myfoosymbol",
    )
}

#[test]
fn hides_all_decls_within_scope() -> TestResult {
    fail_test(
        r#"module myspammodule { export def myfoosymbol [] { "bar" } }; def myfoosymbol [] { "myfoosymbol" }; use myspammodule myfoosymbol; hide myfoosymbol; myfoosymbol"#,
        "external_command",
    )
}

#[test]
fn hides_all_envs_within_scope() -> TestResult {
    fail_test(
        r#"module myspammodule { export-env { $env.myfoosymbol = "bar" } }; $env.myfoosymbol = "myfoosymbol"; use myspammodule; hide-env myfoosymbol; $env.myfoosymbol"#,
        "",
    )
}

#[test]
fn hides_main_import_1() -> TestResult {
    fail_test(
        r#"module myspammodule { export def main [] { "myfoosymbol" } }; use myspammodule; hide myspammodule; myspammodule"#,
        "external_command",
    )
}

#[test]
fn hides_main_import_2() -> TestResult {
    fail_test(
        r#"module myspammodule { export def main [] { "myfoosymbol" } }; use myspammodule; hide myspammodule main; myspammodule"#,
        "external_command",
    )
}

#[test]
fn hides_main_import_3() -> TestResult {
    fail_test(
        r#"module myspammodule { export def main [] { "myfoosymbol" } }; use myspammodule; hide myspammodule [ main ]; myspammodule"#,
        "external_command",
    )
}

#[test]
fn hides_main_import_4() -> TestResult {
    fail_test(
        r#"module myspammodule { export def main [] { "myfoosymbol" } }; use myspammodule; hide myspammodule *; myspammodule"#,
        "external_command",
    )
}
