use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

#[test]
fn view_source_returns_string() -> Result {
    let source = "def foo [] { echo hi }";
    let code = format!("{source}; view source foo");
    test().run(code).expect_value_eq(source)
}

#[test]
fn appends_called_commands() -> Result {
    let code = "def helper [] { 42 }; def caller [] { helper }; view source caller --dependencies";
    test()
        .run(code)
        .expect_value_eq("def caller [] { helper }\n\ndef helper [] { 42 }")
}

#[test]
fn reaches_module_private_commands() -> Result {
    // A private command cannot be named to `view source` at all, so expanding it is the whole point
    // of the flag.
    let code = r#"
        module m2 { def helper [] { 42 }; export def bar [] { helper } }
        use m2
        view source "m2 bar" --dependencies
    "#;
    test()
        .run(code)
        .expect_value_eq("def \"m2 bar\" [] { helper }\n\ndef helper [] { 42 }")
}

#[test]
fn without_the_flag_output_is_unchanged() -> Result {
    let code = "def helper [] { 42 }; def caller [] { helper }; view source caller";
    test().run(code).expect_value_eq("def caller [] { helper }")
}

#[test]
fn skips_builtins() -> Result {
    // Why: the custom dependency has to be there, otherwise this passes with the whole feature
    // removed and proves nothing about builtins.
    let code = "def helper [] { 42 }; def mixed [] { ls | sort-by name; helper }; view source mixed --dependencies";
    test()
        .run(code)
        .expect_value_eq("def mixed [] { ls | sort-by name; helper }\n\ndef helper [] { 42 }")
}

#[test]
fn stops_on_mutual_recursion() -> Result {
    let code = "def a [] { b }; def b [] { a }; view source a --dependencies";
    test()
        .run(code)
        .expect_value_eq("def a [] { b }\n\ndef b [] { a }")
}

#[test]
fn finds_calls_inside_closures() -> Result {
    let code = "def helper [] { 42 }; def caller [] { [1] | each { helper } }; view source caller --dependencies";
    test()
        .run(code)
        .expect_value_eq("def caller [] { [1] | each { helper } }\n\ndef helper [] { 42 }")
}

#[test]
fn follows_an_alias_to_its_target() -> Result {
    // The parser folds an alias call into the target's decl, so the dependency is the target.
    let code = "def t [] { 1 }; alias al = t; def c [] { al }; view source c --dependencies";
    test()
        .run(code)
        .expect_value_eq("def c [] { al }\n\ndef t [] { 1 }")
}

#[test]
fn does_not_repeat_a_nested_def() -> Result {
    // The text of `inner` is already inside the printed body of `outer`. `helper` is here so that
    // removing the feature fails this test instead of satisfying it.
    let code = "def helper [] { 42 }; def outer [] { def inner [] { 1 }; inner; helper }; view source outer --dependencies";
    test().run(code).expect_value_eq(
        "def outer [] { def inner [] { 1 }; inner; helper }\n\ndef helper [] { 42 }",
    )
}

#[test]
fn names_a_dep_the_way_its_call_site_does() -> Result {
    // `main` is imported under the module's name, but the quoted body calls it `main`, so that is
    // the name the header has to use.
    let code = "
        module m3 { export def main [] { 1 }; export def caller [] { main } }
        use m3 *
        view source caller --dependencies
    ";
    test()
        .run(code)
        .expect_value_eq("def caller [] { main }\n\ndef main [] { 1 }")
}

#[test]
fn keeps_metadata_of_the_root() -> Result {
    Playground::setup("view_source_dependencies_metadata", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "mdata.nu",
            "
                def helper [] { 42 }
                def foo [] { helper }
            ",
        )]);

        // Why: assert the dependency is there too, otherwise the metadata alone would still pass
        // with the whole feature removed.
        let expanded: String = test()
            .cwd(dirs.test())
            .run("source mdata.nu; view source foo --dependencies")?;
        assert_contains("def helper [] { 42 }", expanded);

        let code = "
            source mdata.nu
            view source foo --dependencies | metadata | get source
        ";

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        assert_contains("mdata.nu", outcome);
        Ok(())
    })
}

#[test]
fn datasource_filepath_metadata() -> Result {
    Playground::setup("cd_ds_filepath_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "mdata.nu",
            "
                def foo [] { echo hi }
            ",
        )]);

        let code = "
            source mdata.nu
            view source foo | metadata | get source
        ";

        let outcome: String = test().cwd(dirs.test()).run(code)?;
        // expect path printed somehow
        assert_contains("mdata.nu", outcome);
        Ok(())
    })
}

#[test]
fn prepends_constants_the_body_reads() -> Result {
    // Why: constants come first so a body reading `$G` is read after `$G` itself.
    let code = "const G = 42; def caller [] { $G }; view source caller --dependencies";
    test()
        .run(code)
        .expect_value_eq("const G = 42\n\ndef caller [] { $G }")
}

#[test]
fn reaches_module_private_constants() -> Result {
    // A private constant cannot be named to `view source` at all, same as a private command.
    let code = r#"
        module m4 { const P = 7; export def bar [] { $P } }
        use m4
        view source "m4 bar" --dependencies
    "#;
    test()
        .run(code)
        .expect_value_eq("const P = 7\n\ndef \"m4 bar\" [] { $P }")
}

#[test]
fn skips_constants_nobody_wrote_as_one() -> Result {
    // `$nu` is a constant holding the whole record, and printing it would bury the output. The
    // custom dependency is here so that removing the feature fails this test instead of passing it.
    let code = "def helper [] { 42 }; def m [] { $env.PWD; $nu.home-path; helper }; view source m --dependencies";
    test()
        .run(code)
        .expect_value_eq("def m [] { $env.PWD; $nu.home-path; helper }\n\ndef helper [] { 42 }")
}

#[test]
fn skips_the_record_bound_by_use() -> Result {
    // `use m5` binds a record of the module's exported constants under the module's own name. It is
    // a constant, but it is declared at the span of the `use` call, not at a name of its own.
    let code = "
        module m5 { export const E = 1 }
        use m5
        def caller [] { $m5.E }
        view source caller --dependencies
    ";
    test().run(code).expect_value_eq("def caller [] { $m5.E }")
}

#[test]
fn does_not_repeat_a_constant_declared_in_the_body() -> Result {
    // The text of `X` is already inside the printed body. `G` is here so that removing the feature
    // fails this test instead of satisfying it.
    let code = "const G = 1; def foo [] { const X = 2; $X + $G }; view source foo --dependencies";
    test()
        .run(code)
        .expect_value_eq("const G = 1\n\ndef foo [] { const X = 2; $X + $G }")
}

#[test]
fn skips_ordinary_variables() -> Result {
    // Only a constant has a value to show; a parameter and a `let` binding have none until the
    // command runs.
    let code = "def loc [a] { let b = 2; $a + $b }; view source loc --dependencies";
    test()
        .run(code)
        .expect_value_eq("def loc [ a: any ] { let b = 2; $a + $b }")
}

#[test]
fn follows_a_constant_imported_by_name() -> Result {
    // Importing a constant by name reuses the variable the module declared, so it keeps the span of
    // its own `const` -- unlike the record `use m6` alone would bind.
    let code = "
        module m6 { export const E = 5 }
        use m6 E
        def c [] { $E }
        view source c --dependencies
    ";
    test()
        .run(code)
        .expect_value_eq("const E = 5\n\ndef c [] { $E }")
}
