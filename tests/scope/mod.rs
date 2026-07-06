use nu_protocol::test_value;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;

// Note: These tests might slightly overlap with crates/nu-command/tests/commands/help.rs

#[test]
fn scope_shows_alias() -> Result {
    let code = "
        alias xaz = echo alias1
        scope aliases | find xaz | length
    ";
    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_shows_command() -> Result {
    let code = "
        def xaz [] { echo xaz }
        scope commands | find xaz --columns [name] | length
    ";
    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_doesnt_show_scoped_hidden_alias() -> Result {
    let code = "
        alias xaz = echo alias1
        do {
            hide xaz
            scope aliases | find xaz | length
        }
    ";
    test().run(code).expect_value_eq(0)
}

#[test]
fn scope_doesnt_show_hidden_alias() -> Result {
    let code = "
        alias xaz = echo alias1
        hide xaz
        scope aliases | find xaz | length
    ";
    test().run(code).expect_value_eq(0)
}

#[test]
fn scope_doesnt_show_scoped_hidden_command() -> Result {
    let code = "
        def xaz [] { echo xaz }
        do {
            hide xaz
            scope commands | find xaz --columns [name] | length
        }
    ";
    test().run(code).expect_value_eq(0)
}

#[test]
fn scope_doesnt_show_hidden_command() -> Result {
    let code = "
        def xaz [] { echo xaz }
        hide xaz
        scope commands | find xaz --columns [name] | length
    ";
    test().run(code).expect_value_eq(0)
}

// same problem as 'which' command
#[ignore = "See https://github.com/nushell/nushell/issues/4837"]
#[test]
fn correctly_report_of_shadowed_alias() -> Result {
    let code = "
        alias xaz = echo alias1
        def helper [] {
            alias xaz = echo alias2
            scope aliases
        }
        helper | where alias == xaz | get expansion.0
    ";
    test().run(code).expect_value_eq("echo alias 2")
}

#[test]
fn correct_scope_modules_fields() -> Result {
    let module_setup = "
        # nice spam
        #
        # and some extra description for spam

        export module eggs {
            export module bacon {
                export def sausage [] { 'sausage' }
            }
        }

        export def main [] { 'foo' };
        export alias xaz = print
        export extern git []
        export const X = 4

        export-env { $env.SPAM = 'spam' }
    ";
    Playground::setup("correct_scope_modules_fields", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        #[rustfmt::skip]
        let () = tester.run("
            let module = scope modules
            | where name == 'spam'
            | first -s
        ")?;

        tester
            .run("$module | select name description extra_description has_env_block")
            .expect_value_eq(test_value!({
                name: "spam",
                description: "nice spam",
                extra_description: "and some extra description for spam",
                has_env_block: true,
            }))?;

        tester
            .run("$module.commands.0.name")
            .expect_value_eq("spam")?;
        tester
            .run("$module.aliases.0.name")
            .expect_value_eq("xaz")?;
        tester
            .run("$module.externs.0.name")
            .expect_value_eq("git")?;
        tester
            .run("$module.constants.0.name")
            .expect_value_eq("X")?;
        tester
            .run("$module.submodules.0.submodules.0.name")
            .expect_value_eq("bacon")?;
        tester
            .run("$module.submodules.0.submodules.0.commands.0.name")
            .expect_value_eq("sausage")?;

        Ok(())
    })
}

#[test]
fn scope_modules_ignores_leading_shebang_in_module_description() -> Result {
    Playground::setup(
        "scope_modules_ignores_leading_shebang_in_module_description",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "spam.nu",
                "\
#!/usr/bin/env nu

# module_line1
#
# module_line2

export def foo [] {}
",
            )]);

            let mut tester = test().cwd(dirs.test());
            let description: String = tester
                .run("use spam.nu *; scope modules | where name == spam | get 0.description")?;
            assert_eq!(description, "module_line1");
            Ok(())
        },
    )
}

#[test]
fn correct_scope_aliases_fields() -> Result {
    Playground::setup("correct_scope_aliases_fields", |dirs, sandbox| {
        let module_setup = "
            # nice alias
            export alias xaz = print
        ";
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        #[rustfmt::skip]
        let () = tester.run("
            let alias = scope aliases
            | where name == 'spam xaz'
            | first -s
        ")?;

        tester
            .run("$alias | select name expansion description")
            .expect_value_eq(test_value!({
                name: "spam xaz",
                expansion: "print",
                description: "nice alias",
            }))?;

        let _: i64 = tester.run("$alias.decl_id")?;
        let _: i64 = tester.run("$alias.aliased_decl_id")?;

        Ok(())
    })
}

#[test]
fn scope_alias_aliased_decl_id_external() -> Result {
    let code = "
        alias c = cargo
        scope aliases | where name == c | get 0.aliased_decl_id | is-empty
    ";
    test().run(code).expect_value_eq(true)
}

#[test]
fn correct_scope_externs_fields() -> Result {
    Playground::setup("correct_scope_aliases_fields", |dirs, sandbox| {
        let module_setup = "
            # nice extern
            export extern git []
        ";
        sandbox.with_files(&[FileWithContent("spam.nu", module_setup)]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        #[rustfmt::skip]
        let () = tester.run("
            let extern = scope externs
            | where name == 'spam git'
            | first -s
        ")?;

        tester
            .run("$extern | select name description")
            .expect_value_eq(test_value!({
                name: "spam git",
                description: "nice extern",
            }))?;

        let _: i64 = tester.run("$extern.decl_id")?;

        Ok(())
    })
}

#[test]
fn scope_externs_sorted() -> Result {
    let code = "
        extern a []
        extern b []
        extern c []
        scope externs | get name
    ";
    test().run(code).expect_value_eq(["a", "b", "c"])
}

#[test]
fn correct_scope_variables_fields() -> Result {
    let code = r#"
        let x = "x val"

        let x_var = scope variables | where name == '$x' | first -s
    "#;

    let mut tester = test();
    let () = tester.run(code)?;

    tester
        .run("$x_var | select name type value is_const")
        .expect_value_eq(test_value!({
            name: "$x",
            "type": "string",
            value: "x val",
            is_const: false,
        }))?;
    let _: i64 = tester.run("$x_var.var_id")?;

    let code = r#"
        const x = 'x'
        scope variables | where name == '$x' | get 0.is_const
    "#;
    test().run(code).expect_value_eq(true)?;

    Ok(())
}

#[test]
fn example_results_have_valid_span() -> Result {
    let code = "
        scope commands
        | where name == 'do'
        | first
        | get examples
        | where result == 177
        | get 0.result
        | metadata
        | view span $in.span.start $in.span.end
    ";
    test().run(code).expect_value_eq("scope commands")
}
