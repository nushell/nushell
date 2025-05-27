use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, nu_with_std};
use pretty_assertions::assert_eq;

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
fn hides_environment_from_child() {
    let actual = nu!(r#"
        $env.TEST = 1; ^$nu.current-exe -c "hide-env TEST; ^$nu.current-exe -c '$env.TEST'"
    "#);
    assert!(actual.out.is_empty());
    assert!(actual.err.contains("column_not_found") || actual.err.contains("name_not_found"));
}

#[test]
fn has_file_pwd() {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", "$env.FILE_PWD")]);

        let actual = nu!(cwd: dirs.test(), "nu spam.nu");

        assert!(actual.out.ends_with("has_file_pwd"));
    })
}

#[test]
fn has_file_loc() {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", "$env.CURRENT_FILE")]);

        let actual = nu!(cwd: dirs.test(), "nu spam.nu");

        assert!(actual.out.ends_with("spam.nu"));
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

#[test]
fn env_var_case_insensitive() {
    let actual = nu!("
        $env.foo = 111
        print $env.Foo
        $env.FOO = 222
        print $env.foo
    ");
    assert!(actual.out.contains("111"));
    assert!(actual.out.contains("222"));
}

#[test]
fn env_conversion_on_assignment() {
    let actual = nu!(r#"
        $env.FOO = "bar:baz:quox"
        $env.ENV_CONVERSIONS = { FOO: { from_string: {|| split row ":"} } }
        $env.FOO | to nuon
    "#);
    assert_eq!(actual.out, "[bar, baz, quox]");
}

#[test]
fn std_log_env_vars_are_not_overridden() {
    let actual = nu_with_std!(
        envs: vec![
            ("NU_LOG_FORMAT".to_string(), "%MSG%".to_string()),
            ("NU_LOG_DATE_FORMAT".to_string(), "%Y".to_string()),
        ],
        r#"
            use std/log
            print -e $env.NU_LOG_FORMAT
            print -e $env.NU_LOG_DATE_FORMAT
            log error "err"
        "#
    );
    assert_eq!(actual.err, "%MSG%\n%Y\nerr\n");
}

#[test]
fn std_log_env_vars_have_defaults() {
    let actual = nu_with_std!(
        r#"
            use std/log
            print -e $env.NU_LOG_FORMAT
            print -e $env.NU_LOG_DATE_FORMAT
        "#
    );
    assert!(actual.err.contains("%MSG%"));
    assert!(actual.err.contains("%Y-"));
}

#[test]
fn env_shlvl_commandstring_does_not_increment() {
    let actual = nu!("
        $env.SHLVL = 5
        nu -c 'print $env.SHLVL; exit'
    ");

    assert_eq!(actual.out, "5");
}

// Note: Do not use -i / --interactive in tests.
// -i attempts to acquire a terminal, and if more than one
// test tries to obtain a terminal at the same time, the
// test run will likely hang, at least for some users.
// Instead, use -e / --execute with an `exit` to test REPL
// functionality as demonstrated below.
//
// We've also learned that `-e 'exit'` is not enough to
// prevent failures entirely. For now we're going to ignore
// these tests until we can find a better solution.
#[ignore = "Causing hangs when both tests overlap"]
#[test]
fn env_shlvl_in_repl() {
    let actual = nu!("
        $env.SHLVL = 5
        nu --no-std-lib -n -e 'print $env.SHLVL; exit'
    ");

    assert_eq!(actual.out, "6");
}

#[ignore = "Causing hangs when both tests overlap"]
#[test]
fn env_shlvl_in_exec_repl() {
    let actual = nu!(r#"
        $env.SHLVL = 29
        nu -c "exec nu --no-std-lib -n -e 'print $env.SHLVL; exit'"
    "#);

    assert_eq!(actual.out, "30");
}

#[test]
fn path_is_a_list_in_repl() {
    let actual = nu!(r#"
        nu -c "exec nu --no-std-lib -n -e 'print ($env.pATh | describe); exit'"
    "#);

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn path_is_a_list() {
    let actual = nu!("
        print ($env.path | describe)
    ");

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn path_is_a_list_in_script() {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("checkpath.nu", "$env.path | describe")]);

        let actual = nu!(cwd: dirs.test(), "nu checkpath.nu");

        assert!(actual.out.ends_with("list<string>"));
    })
}
