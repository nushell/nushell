use super::support::Trusted;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code};
use pretty_assertions::assert_eq;
use serial_test::serial;

#[test]
fn env_shorthand() {
    let actual = nu!("
        FOO=bar echo $env.FOO
        ");
    assert_eq!(actual.out, "bar");
}

#[test]
fn env_shorthand_with_equals() {
    let actual = nu!("
        RUST_LOG=my_module=info $env.RUST_LOG
    ");
    assert_eq!(actual.out, "my_module=info");
}

#[test]
fn env_shorthand_with_interpolation() {
    let actual = nu!(r#"
        let num = 123
        FOO=$"($num) bar" echo $env.FOO
        "#);
    assert_eq!(actual.out, "123 bar");
}

#[test]
fn env_shorthand_with_comma_equals() {
    let actual = nu!("
        RUST_LOG=info,my_module=info $env.RUST_LOG
    ");
    assert_eq!(actual.out, "info,my_module=info");
}

#[test]
fn env_shorthand_with_comma_colons_equals() {
    let actual = nu!("
        RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.RUST_LOG
    ");
    assert_eq!(actual.out, "info,my_module=info,lib_crate::lib_mod=trace");
}

#[test]
fn env_shorthand_multi_second_with_comma_colons_equals() {
    let actual = nu!("
        FOO=bar RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.FOO + $env.RUST_LOG
    ");
    assert_eq!(
        actual.out,
        "barinfo,my_module=info,lib_crate::lib_mod=trace"
    );
}

#[test]
fn env_shorthand_multi_first_with_comma_colons_equals() {
    let actual = nu!("
        RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace FOO=bar $env.FOO + $env.RUST_LOG
    ");
    assert_eq!(
        actual.out,
        "barinfo,my_module=info,lib_crate::lib_mod=trace"
    );
}

#[test]
fn env_shorthand_multi() {
    let actual = nu!("
        FOO=bar BAR=baz $env.FOO + $env.BAR
    ");
    assert_eq!(actual.out, "barbaz");
}

#[test]
fn env_assignment() {
    let actual = nu!(r#"
        $env.FOOBAR = "barbaz"; $env.FOOBAR
    "#);
    assert_eq!(actual.out, "barbaz");
}

#[test]
fn env_assignment_with_if() {
    let actual = nu!(r#"$env.FOOBAR = if 3 == 4 { "bar" } else { "baz" }; $env.FOOBAR"#);
    assert_eq!(actual.out, "baz");
}

#[test]
fn env_assignment_with_match() {
    let actual = nu!(r#"$env.FOOBAR = match 1 { 1 => { 'yes!' }, _ => { 'no!' } }; $env.FOOBAR"#);
    assert_eq!(actual.out, "yes!");
}

#[test]
fn mutate_env_file_pwd_env_var_fails() {
    let actual = nu!("$env.FILE_PWD = 'foo'");

    assert!(actual.err.contains("automatic_env_var_set_manually"));
}

#[test]
fn load_env_file_pwd_env_var_fails() {
    let actual = nu!("load-env { FILE_PWD : 'foo' }");

    assert!(actual.err.contains("automatic_env_var_set_manually"));
}

#[test]
fn load_env_pwd_env_var_fails() {
    let actual = nu!("load-env { PWD : 'foo' }");

    assert!(actual.err.contains("automatic_env_var_set_manually"));
}

#[test]
fn passes_with_env_env_var_to_external_process() {
    let actual = nu!("
        with-env { FOO: foo } {nu --testbin echo_env FOO}
        ");
    assert_eq!(actual.out, "foo");
}

#[test]
fn has_file_pwd() {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent("spam.nu", "$env.FILE_PWD")]);

        let actual = nu!(cwd: dirs.test(), "nu spam.nu");

        assert!(actual.out.ends_with("has_file_pwd"));
    })
}

#[test]
fn has_file_loc() {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent("spam.nu", "$env.CURRENT_FILE")]);

        let actual = nu!(cwd: dirs.test(), "nu spam.nu");

        assert!(actual.out.ends_with("spam.nu"));
    })
}

// FIXME: autoenv not currently implemented
#[ignore]
#[test]
#[serial]
fn passes_env_from_local_cfg_to_external_process() {
    Playground::setup("autoenv_dir", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            r#"[env]
            FOO = "foo"
            "#,
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), "
                nu --testbin echo_env FOO
            ")
        });

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn hides_env_in_block() {
    let inp = &[
        "$env.foo = 'foo'",
        "hide-env foo",
        "let b = {|| $env.foo }",
        "do $b",
    ];

    let actual = nu!(&inp.join("; "));
    let actual_repl = nu!(nu_repl_code(inp));

    assert!(actual.err.contains("column_not_found"));
    assert!(actual_repl.err.contains("column_not_found"));
}

#[test]
fn env_var_not_var() {
    let actual = nu!("
        echo $PWD
        ");
    assert!(actual.err.contains("use $env.PWD instead of $PWD"));
}
