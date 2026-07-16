use nu_test_support::prelude::*;

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn get_env_by_name() -> Result {
    let mut tester = test();
    let () = tester.run("$env.FOO = 'bar'")?;
    tester.run("example env FOO").expect_value_eq("bar")?;
    let () = tester.run("$env.FOO = 'baz'")?;
    tester.run("example env FOO").expect_value_eq("baz")?;
    Ok(())
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn get_envs() -> Result {
    test()
        .run("$env.BAZ = 'foo'; example env | get BAZ")
        .expect_value_eq("foo")
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn get_current_dir() -> Result {
    Playground::setup(&module_path!().replace("::", "_"), |_, playground| {
        playground.mkdir("tests");
        test()
            .cwd(playground.cwd())
            .run("cd tests; example env --cwd")
            .expect_value_eq(playground.cwd().join("tests"))
    })
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn set_env() -> Result {
    test()
        .run("example env NUSHELL_OPINION --set=rocks; $env.NUSHELL_OPINION")
        .expect_value_eq("rocks")
}
