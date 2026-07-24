use nu_test_support::{fs::Stub::FileWithContent, prelude::*};
use pretty_assertions::assert_eq;

#[test]
fn env_shorthand() -> Result {
    test().run("FOO=bar echo $env.FOO").expect_value_eq("bar")
}

#[test]
fn env_shorthand_with_equals() -> Result {
    test()
        .run("RUST_LOG=my_module=info $env.RUST_LOG")
        .expect_value_eq("my_module=info")
}

#[test]
fn env_shorthand_with_interpolation() -> Result {
    let code = r#"
        let num = 123
        FOO=$"($num) bar" echo $env.FOO
    "#;

    test().run(code).expect_value_eq("123 bar")
}

#[test]
fn env_shorthand_with_comma_equals() -> Result {
    test()
        .run("RUST_LOG=info,my_module=info $env.RUST_LOG")
        .expect_value_eq("info,my_module=info")
}

#[test]
fn env_shorthand_with_comma_colons_equals() -> Result {
    test()
        .run("RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.RUST_LOG")
        .expect_value_eq("info,my_module=info,lib_crate::lib_mod=trace")
}

#[test]
fn env_shorthand_multi_second_with_comma_colons_equals() -> Result {
    test()
        .run("FOO=bar RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace $env.FOO + $env.RUST_LOG")
        .expect_value_eq("barinfo,my_module=info,lib_crate::lib_mod=trace")
}

#[test]
fn env_shorthand_multi_first_with_comma_colons_equals() -> Result {
    test()
        .run("RUST_LOG=info,my_module=info,lib_crate::lib_mod=trace FOO=bar $env.FOO + $env.RUST_LOG")
        .expect_value_eq("barinfo,my_module=info,lib_crate::lib_mod=trace")
}

#[test]
fn env_shorthand_multi() -> Result {
    test()
        .run("FOO=bar BAR=baz $env.FOO + $env.BAR")
        .expect_value_eq("barbaz")
}

#[test]
fn env_assignment() -> Result {
    test()
        .run(r#"$env.FOOBAR = "barbaz"; $env.FOOBAR"#)
        .expect_value_eq("barbaz")
}

#[test]
fn env_assignment_with_if() -> Result {
    test()
        .run(r#"$env.FOOBAR = if 3 == 4 { "bar" } else { "baz" }; $env.FOOBAR"#)
        .expect_value_eq("baz")
}

#[test]
fn env_assignment_with_match() -> Result {
    test()
        .run("$env.FOOBAR = match 1 { 1 => { 'yes!' }, _ => { 'no!' } }; $env.FOOBAR")
        .expect_value_eq("yes!")
}

#[test]
fn mutate_env_file_pwd_env_var_fails() -> Result {
    test()
        .run("$env.FILE_PWD = 'foo'")
        .expect_error_code_eq("nu::compile::automatic_env_var_set_manually")
}

#[test]
fn load_env_file_pwd_env_var_fails() -> Result {
    test()
        .run("load-env { FILE_PWD : 'foo' }")
        .expect_error_code_eq("nu::shell::automatic_env_var_set_manually")
}

#[test]
fn load_env_pwd_env_var_fails() -> Result {
    test()
        .run("load-env { PWD : 'foo' }")
        .expect_error_code_eq("nu::shell::automatic_env_var_set_manually")
}

#[test]
#[deps(TESTBIN_ECHO_ENV)]
fn passes_with_env_env_var_to_external_process() -> Result {
    test()
        .run("with-env { FOO: foo } { echo_env FOO }")
        .expect_value_eq("foo")
}

#[test]
#[deps(NU)]
fn hides_environment_from_child() -> Result {
    let result: CompleteResult = test()
        .env("TEST", 1)
        .run(r#"nu -c 'hide-env TEST; nu -c "$env.TEST"' | complete"#)?;

    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("column_not_found") || result.stderr.contains("name_not_found"));
    Ok(())
}

#[test]
#[deps(NU)]
fn has_file_pwd() -> Result {
    Playground::setup("has_file_pwd", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", "$env.FILE_PWD")]);

        test()
            .cwd(dirs.test())
            .run("nu spam.nu | to text | str trim")
            .expect_value_eq(dirs.test().to_string_lossy())
    })
}

#[test]
#[deps(NU)]
fn has_file_loc() -> Result {
    Playground::setup("has_file_loc", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", "$env.CURRENT_FILE")]);

        let actual: String = test()
            .cwd(dirs.test())
            .run("nu spam.nu | to text | str trim")?;

        assert!(actual.ends_with("spam.nu"));
        Ok(())
    })
}

#[test]
fn hides_env_in_block() -> Result {
    let pipelines = [
        "$env.foo = 'foo'",
        "hide-env foo",
        "let b = {|| $env.foo }",
        "do $b",
    ];

    test()
        .run(pipelines.join("; "))
        .expect_error_code_eq("nu::shell::column_not_found")?;

    test()
        .run_multiple(pipelines)
        .expect_error_code_eq("nu::shell::column_not_found")
}

#[test]
fn env_var_not_var() -> Result {
    let err = test().run("echo $PWD").expect_parse_error()?;
    assert_contains("Use $env.PWD instead of $PWD", err.to_string());
    Ok(())
}

#[test]
fn env_var_case_insensitive() -> Result {
    let code = "
        $env.foo = 111
        let first = $env.Foo
        $env.FOO = 222
        [$first, $env.foo]
    ";

    test().run(code).expect_value_eq([111, 222])
}

#[test]
fn env_conversion_on_assignment() -> Result {
    let code = r#"
        $env.FOO = "bar:baz:quox"
        $env.ENV_CONVERSIONS = { FOO: { from_string: {|| split row ":"} } }
        $env.FOO
    "#;

    test().run(code).expect_value_eq(["bar", "baz", "quox"])
}

#[test]
#[deps(NU)]
fn std_log_env_vars_are_not_overridden() -> Result {
    let result: CompleteResult = test()
        .env("NU_LOG_FORMAT", "%MSG%")
        .env("NU_LOG_DATE_FORMAT", "%Y")
        .run(
            r#"
                nu -n -c '
                    use std/log
                    print -e $env.NU_LOG_FORMAT
                    print -e $env.NU_LOG_DATE_FORMAT
                    log error "err"
                ' | complete
            "#,
        )?;

    assert_eq!(result.stderr, "%MSG%\n%Y\nerr\n");
    Ok(())
}

#[test]
#[deps(NU)]
fn std_log_env_vars_have_defaults() -> Result {
    let result: CompleteResult = test().run(
        "
                nu -n -c '
                    use std/log
                    print -e $env.NU_LOG_FORMAT
                    print -e $env.NU_LOG_DATE_FORMAT
                ' | complete
            ",
    )?;

    assert_contains("%MSG%", &result.stderr);
    assert_contains("%Y-", &result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn env_shlvl_commandstring_does_not_increment() -> Result {
    test()
        .env("SHLVL", 5)
        .run("nu -c '$env.SHLVL | to text | str trim'")
        .expect_value_eq("5")
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
#[test]
#[deps(NU)]
fn env_shlvl_in_repl() -> Result {
    let out: String = test()
        .env("SHLVL", 5)
        .run(r#"nu --no-std-lib -n -e 'print $"SHLVL:($env.SHLVL)"; exit' | to text"#)?;

    assert!(out.trim_end().ends_with("SHLVL:6"));
    Ok(())
}

#[test]
#[deps(NU)]
fn env_shlvl_in_exec_repl() -> Result {
    let out: String = test().env("SHLVL", 29).run(
        r#"nu -c 'exec nu --no-std-lib -n -e `print $"SHLVL:($env.SHLVL)"; exit`' | to text"#,
    )?;

    assert!(out.trim_end().ends_with("SHLVL:30"));
    Ok(())
}

#[test]
#[deps(NU)]
fn path_is_a_list_in_repl() -> Result {
    test()
        .run(r#"nu -c "exec nu --no-std-lib -n -e `print $'path:($env.pATh | describe)'; exit`" | to text | str trim"#)
        .expect_value_eq("path:list<string>")
}

#[test]
#[deps(NU)]
fn path_is_a_list() -> Result {
    test()
        .run("nu -c '$env.path | describe' | to text | str trim")
        .expect_value_eq("list<string>")
}

#[test]
#[deps(NU)]
fn path_is_a_list_in_script() -> Result {
    Playground::setup("path_is_a_list_in_script", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("checkpath.nu", "$env.path | describe")]);

        test()
            .cwd(dirs.test())
            .run("nu checkpath.nu | to text | str trim")
            .expect_value_eq("list<string>")
    })
}

#[test]
fn case_insensitive_env_load_env() -> Result {
    let code = "
        load-env {testvar: 'value1', TESTVAR: 'value2'}
        [$env.testvar, $env.TESTVAR]
    ";

    test().run(code).expect_value_eq(["value2", "value2"])
}

#[test]
fn case_insensitive_env_http_proxy() -> Result {
    test()
        .run("$env.http_proxy = 'http://proxy.example.com'; $env.HTTP_PROXY")
        .expect_value_eq("http://proxy.example.com")
}

#[test]
fn case_insensitive_env_date_locale() -> Result {
    test()
        .run("$env.lc_all = 'C'; $env.LC_ALL")
        .expect_value_eq("C")
}

#[test]
fn case_insensitive_env_record_access() -> Result {
    test()
        .run("$env.test = 'value'; $env.TEST")
        .expect_value_eq("value")
}
