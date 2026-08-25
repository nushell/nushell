use nu_protocol::test_value;
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
fn correct_scope_modules_fields(playground: Playground) -> Result {
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
    playground.file("spam.nu", module_setup)?;

    let mut tester = test().cwd(playground.path());
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
}

#[test]
fn scope_modules_ignores_leading_shebang_in_module_description(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "\
                #!/usr/bin/env nu
                
                # module_line1
                #
                # module_line2
                
                export def foo [] {}
            ",
    )?;

    let mut tester = test().cwd(playground.path());
    let description: String =
        tester.run("use spam.nu *; scope modules | where name == spam | get 0.description")?;
    assert_eq!(description, "module_line1");
    Ok(())
}

#[test]
fn correct_scope_aliases_fields(playground: Playground) -> Result {
    let module_setup = "
        # nice alias
        export alias xaz = print
    ";
    playground.file("spam.nu", module_setup)?;

    let mut tester = test().cwd(playground.path());
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
fn correct_scope_externs_fields(playground: Playground) -> Result {
    let module_setup = "
        # nice extern
        export extern git []
    ";
    playground.file("spam.nu", module_setup)?;

    let mut tester = test().cwd(playground.path());
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

    let code = "
        const x = 'x'
        scope variables | where name == '$x' | get 0.is_const
    ";
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

// --- Local scope visibility (#14071) ---

#[test]
fn scope_variables_shows_locals_in_closure() -> Result {
    let code = "
        do {
            let b = 2
            scope variables | where name == '$b' | get 0.value
        }
    ";

    test().run(code).expect_value_eq(2)
}

#[test]
fn scope_variables_shows_outer_and_local_in_closure() -> Result {
    // Issue #14071: inner scope should still see the global/outer scope.
    let code = "
        let a = 1
        do {
            let b = 2
            scope variables | where name in ['$a', '$b'] | get name | sort
        }
    ";

    test().run(code).expect_value_eq(["$a", "$b"])?;

    let code = "
        let a = 1
        do {
            let b = 2
            [
                (scope variables | where name == '$a' | get 0.value)
                (scope variables | where name == '$b' | get 0.value)
            ]
        }
    ";

    test().run(code).expect_value_eq([1, 2])
}

#[test]
fn scope_variables_shows_locals_in_block() -> Result {
    let code = "
        let a = 1
        if true {
            let c = 3
            scope variables | where name == '$c' | get 0.value
        }
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
fn scope_variables_shows_outer_and_local_in_block() -> Result {
    let code = "
        let a = 1
        if true {
            let c = 3
            scope variables | where name in ['$a', '$c'] | get name | sort
        }
    ";

    test().run(code).expect_value_eq(["$a", "$c"])
}

#[test]
fn scope_variables_shows_for_loop_var() -> Result {
    // `for` does not return the last pipeline value of its block.
    let code = "
        mut val = -1
        for i in 1..1 {
            $val = (scope variables | where name == '$i' | get 0.value)
        }
        $val
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_variables_shows_def_params() -> Result {
    let code = "
        def f [x] {
            scope variables | where name == '$x' | get 0.value
        }
        f 42
    ";

    test().run(code).expect_value_eq(42)
}

#[test]
fn scope_variables_shadowed_let_shows_current_value() -> Result {
    // Related to #17414: name maps keep the final VarId, but the live value is earlier.
    let code = "
        let x = 'first'
        let seen = (scope variables | where name == '$x' | get 0.value)
        let x = 'second'
        [$seen $x]
    ";

    test().run(code).expect_value_eq(["first", "second"])
}

#[test]
fn scope_commands_shows_local_def_in_closure() -> Result {
    let code = "
        do {
            def local-cmd [] { 'hi' }
            scope commands | where name == 'local-cmd' | length
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_commands_local_def_not_visible_after_closure() -> Result {
    let code = "
        do { def local-cmd [] { 'hi' } }
        scope commands | where name == 'local-cmd' | length
    ";

    test().run(code).expect_value_eq(0)
}

#[test]
fn scope_aliases_shows_local_alias_in_closure() -> Result {
    let code = "
        do {
            alias la = ls
            scope aliases | where name == 'la' | length
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_modules_shows_local_use_in_closure(playground: Playground) -> Result {
    playground.file("spam.nu", "export def foo [] { 'foo' }")?;

    let code = "
        do {
            use spam.nu
            scope modules | where name == 'spam' | length
        }
    ";

    test().cwd(playground.path()).run(code).expect_value_eq(1)
}

#[test]
fn scope_commands_nested_closure_sees_outer_local_def() -> Result {
    let code = "
        do {
            def outer-cmd [] { 'hi' }
            do {
                scope commands | where name == 'outer-cmd' | length
            }
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_commands_shows_local_def_in_if_block() -> Result {
    let code = "
        if true {
            def local-cmd [] { 'hi' }
            scope commands | where name == 'local-cmd' | length
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_commands_local_def_not_visible_after_if_block() -> Result {
    let code = "
        if true {
            def local-cmd [] { 'hi' }
        }
        scope commands | where name == 'local-cmd' | length
    ";

    test().run(code).expect_value_eq(0)
}

#[test]
fn scope_commands_shows_local_def_in_for_block() -> Result {
    let code = "
        mut n = 0
        for i in 1..1 {
            def local-cmd [] { 'hi' }
            $n = (scope commands | where name == 'local-cmd' | length)
        }
        $n
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_externs_shows_local_extern_in_closure() -> Result {
    let code = "
        do {
            extern local-ext []
            scope externs | where name == 'local-ext' | length
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_modules_shows_local_use_in_if_block(playground: Playground) -> Result {
    playground.file("spam.nu", "export def foo [] { 'foo' }")?;

    let code = "
        if true {
            use spam.nu
            scope modules | where name == 'spam' | length
        }
    ";

    test().cwd(playground.path()).run(code).expect_value_eq(1)
}

#[test]
fn scope_aliases_shows_local_alias_in_if_block() -> Result {
    let code = "
        if true {
            alias la = ls
            scope aliases | where name == 'la' | length
        }
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn scope_commands_shows_deprecated_command() -> Result {
    let code = "
        if true {
            @deprecated --since '0.114.2'
            def depr [] {}
            scope commands | where name == 'depr' | get 0?.deprecation_info.type?
        }
    ";

    test().run(code).expect_value_eq(["Command"])
}

#[test]
fn scope_commands_shows_deprecated_flags() -> Result {
    let code = "
        if true {
            @deprecated --flag foo
            @deprecated --flag bar
            def depr [--foo, --bar, --baz] {}

            scope commands
            | where name == 'depr'
            | get 0?.deprecation_info
            | each {|entry|
                $entry.type == 'Flag' and $entry.flag != null
            }
        }
    ";
    test().run(code).expect_value_eq([true, true])
}
