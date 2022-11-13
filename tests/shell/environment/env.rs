use super::support::Trusted;

use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

use serial_test::serial;

#[test]
fn env_shorthand() {
    let actual = nu!(cwd: ".", r#"
        FOO=bar echo $env.FOO
        "#);
    assert_eq!(actual.out, "bar");
}

#[test]
fn env_shorthand_with_equals() {
    let actual = nu!(cwd: ".", r#"
        RUST_LOG=my_module=info $env.RUST_LOG
    "#);
    assert_eq!(actual.out, "my_module=info");
}

#[test]
fn env_shorthand_with_interpolation() {
    let actual = nu!(cwd: ".", r#"
        let num = 123
        FOO=$"($num) bar" echo $env.FOO
        "#);
    assert_eq!(actual.out, "123 bar");
}

#[test]
fn env_shorthand_with_comma_equals() {
    let actual = nu!(cwd: ".", r#"
        RUST_LOG=info,my_module=info $env.RUST_LOG
    "#);
    assert_eq!(actual.out, "info,my_module=info");
}

#[test]
fn env_shorthand_with_comma_colons_equals() {
    let actual = nu!(cwd: ".", r#"
        RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.RUST_LOG
    "#);
    assert_eq!(actual.out, "info,my_module=info,lib_crate::lib_mod=trace");
}

#[test]
fn env_shorthand_multi_second_with_comma_colons_equals() {
    let actual = nu!(cwd: ".", r#"
        FOO=bar RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.FOO + $env.RUST_LOG
    "#);
    assert_eq!(
        actual.out,
        "barinfo,my_module=info,lib_crate::lib_mod=trace"
    );
}

#[test]
fn env_shorthand_multi_first_with_comma_colons_equals() {
    let actual = nu!(cwd: ".", r#"
        RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace FOO=bar $env.FOO + $env.RUST_LOG
    "#);
    assert_eq!(
        actual.out,
        "barinfo,my_module=info,lib_crate::lib_mod=trace"
    );
}

#[test]
fn env_shorthand_multi() {
    let actual = nu!(cwd: ".", r#"
        FOO=bar BAR=baz $env.FOO + $env.BAR
    "#);
    assert_eq!(actual.out, "barbaz");
}

#[test]
fn env_assignment() {
    let actual = nu!(cwd: ".", r#"
        $env.FOOBAR = "barbaz"; $env.FOOBAR
    "#);
    assert_eq!(actual.out, "barbaz");
}

#[test]
fn let_env_file_pwd_env_var_fails() {
    let actual = nu!(cwd: ".", r#"let-env FILE_PWD = 'foo'"#);

    assert!(actual.err.contains("automatic_env_var_set_manually"));
}

#[test]
fn load_env_file_pwd_env_var_fails() {
    let actual = nu!(cwd: ".", r#"load-env { FILE_PWD : 'foo' }"#);

    assert!(actual.err.contains("automatic_env_var_set_manually"));
}

// FIXME: for some reason Nu is attempting to execute foo in `let-env FOO = foo`
#[ignore]
#[test]
fn passes_let_env_env_var_to_external_process() {
    let actual = nu!(cwd: ".", r#"
        let-env FOO = foo
        nu --testbin echo_env FOO
        "#);
    assert_eq!(actual.out, "foo");
}

#[test]
fn passes_with_env_env_var_to_external_process() {
    let actual = nu!(cwd: ".", r#"
        with-env [FOO foo] {nu --testbin echo_env FOO}
        "#);
    assert_eq!(actual.out, "foo");
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
            nu!(cwd: dirs.test(), r#"
                nu --testbin echo_env FOO
            "#)
        });

        assert_eq!(actual.out, "foo");
    })
}
