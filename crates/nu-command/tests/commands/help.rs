use nu_experimental::NATIVE_CLIP;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use rstest::rstest;

// Note: These tests might slightly overlap with tests/scope/mod.rs

#[test]
fn help_commands_length() -> Result {
    test()
        .run("help commands | length | $in > 0")
        .expect_value_eq(true)
}

#[test]
fn help_shows_signature() -> Result {
    let mut tester = test();

    let outcome: String = tester.run("help str distance")?;
    assert_contains("Input/output types", &outcome);

    // don't show signature for parser keyword
    let outcome: String = tester.run("help alias")?;
    assert!(!outcome.contains("Input/output types"));

    Ok(())
}

#[test]
fn help_aliases() -> Result {
    let mut tester = test();
    let () = tester.run("alias SPAM = print 'spam'")?;
    tester
        .run("help aliases | where name == SPAM | length")
        .expect_value_eq(1)
}

#[test]
fn help_alias_description_1() -> Result {
    Playground::setup("help_alias_description_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                alias SPAM = print 'spam'
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        tester
            .run("help aliases | where name == SPAM | get 0.description")
            .expect_value_eq("line1")
    })
}

#[test]
fn help_alias_description_2() -> Result {
    let mut tester = test();
    let () = tester.run("alias SPAM = print 'spam'  # line2")?;
    tester
        .run("help aliases | where name == SPAM | get 0.description")
        .expect_value_eq("line2")
}

#[test]
fn help_alias_description_3() -> Result {
    Playground::setup("help_alias_description_3", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                alias SPAM = print 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String =
            tester.run("help aliases | where name == SPAM | get 0.description")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        Ok(())
    })
}

#[test]
fn help_alias_name() -> Result {
    // Regression test for #18351 — ensure the alias name "SPAM" appears in
    // `help aliases SPAM` output, not the target command name "print"
    Playground::setup("help_alias_name", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                alias SPAM = print 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help aliases SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("SPAM", &outcome);
        assert_contains("print 'spam'", &outcome);
        Ok(())
    })
}

#[test]
fn help_alias_name_f() -> Result {
    Playground::setup("help_alias_name_f", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                alias SPAM = print 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help aliases -f SPAM | get 0.description")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        Ok(())
    })
}

#[test]
fn help_export_alias_name_single_word() -> Result {
    // #18351: same as help_alias_name but for export alias
    Playground::setup("help_export_alias_name_single_word", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                export alias SPAM = print 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu SPAM")?;
        let outcome: String = tester.run("help aliases SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("SPAM", &outcome);
        assert_contains("print 'spam'", &outcome);
        Ok(())
    })
}

#[test]
fn help_export_alias_name_multi_word() -> Result {
    // #18351: same as help_alias_name but for multi-word export alias
    Playground::setup("help_export_alias_name_multi_word", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                export alias SPAM = print 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        let outcome: String = tester.run("help aliases spam SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("SPAM", &outcome);
        assert_contains("print 'spam'", &outcome);
        Ok(())
    })
}

#[test]
fn help_alias_name_external() -> Result {
    Playground::setup("help_alias_name_external", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                alias SPAM = ^echo 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help aliases SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("SPAM", &outcome);
        assert_contains("^echo 'spam'", &outcome);
        Ok(())
    })
}

#[test]
fn help_export_alias_name_external() -> Result {
    Playground::setup("help_export_alias_name_external", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                export alias SPAM = ^echo 'spam' # line2
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu SPAM")?;
        let outcome: String = tester.run("help aliases SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("SPAM", &outcome);
        assert_contains("^echo 'spam'", &outcome);
        Ok(())
    })
}

#[test]
fn help_module_description_1() -> Result {
    Playground::setup("help_module_description", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                module SPAM {
                    # line2
                } #line3
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String =
            tester.run("help modules | where name == SPAM | get 0.description")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("line3", &outcome);
        Ok(())
    })
}

#[test]
fn help_module_description_ignores_leading_shebang() -> Result {
    Playground::setup(
        "help_module_description_ignores_leading_shebang",
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
                .run("use spam.nu *; help modules | where name == spam | get 0.description")?;
            assert_eq!(description, "module_line1");
            Ok(())
        },
    )
}

#[test]
fn help_module_name() -> Result {
    Playground::setup("help_module_name", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # line1
                module SPAM {
                    # line2
                } #line3
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help modules SPAM")?;

        assert_contains("line1", &outcome);
        assert_contains("line2", &outcome);
        assert_contains("line3", &outcome);
        assert_contains("SPAM", &outcome);
        Ok(())
    })
}

#[test]
fn help_module_sorted_decls() -> Result {
    Playground::setup("help_module_sorted_decls", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                module SPAM {
                    export def z [] {}
                    export def a [] {}
                }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help modules SPAM")?;

        assert_contains("a, z", &outcome);
        Ok(())
    })
}

#[test]
fn help_module_sorted_aliases() -> Result {
    Playground::setup("help_module_sorted_aliases", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                module SPAM {
                    export alias z = echo 'z'
                    export alias a = echo 'a'
                }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("source spam.nu")?;
        let outcome: String = tester.run("help modules SPAM")?;

        assert_contains("a, z", &outcome);
        Ok(())
    })
}

#[test]
fn help_description_extra_description_command() -> Result {
    Playground::setup(
        "help_description_extra_description_command",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "spam.nu",
                "
                # module_line1
                #
                # module_line2

                # def_line1
                #
                # def_line2
                export def foo [] {}
            ",
            )]);
            let mut tester = test().cwd(dirs.test());
            let () = tester.run("use spam.nu *")?;

            let outcome: String = tester.run("help modules spam")?;
            assert_contains("module_line1", &outcome);
            assert_contains("module_line2", &outcome);

            let outcome: String =
                tester.run("help modules | where name == spam | get 0.description")?;
            assert_contains("module_line1", &outcome);
            assert!(!outcome.contains("module_line2"));

            let outcome: String = tester.run("help commands foo")?;
            assert_contains("def_line1", &outcome);
            assert_contains("def_line2", &outcome);

            let outcome: String =
                tester.run("help commands | where name == foo | get 0.description")?;
            assert_contains("def_line1", &outcome);
            assert!(!outcome.contains("def_line2"));
            Ok(())
        },
    )
}

#[test]
fn help_description_extra_description_alias() -> Result {
    Playground::setup(
        "help_description_extra_description_alias",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "spam.nu",
                "
                # module_line1
                #
                # module_line2

                # alias_line1
                #
                # alias_line2
                export alias bar = echo 'bar'
            ",
            )]);
            let mut tester = test().cwd(dirs.test());
            let () = tester.run("use spam.nu *")?;

            let outcome: String = tester.run("help modules spam")?;
            assert_contains("module_line1", &outcome);
            assert_contains("module_line2", &outcome);

            let outcome: String =
                tester.run("help modules | where name == spam | get 0.description")?;
            assert_contains("module_line1", &outcome);
            assert!(!outcome.contains("module_line2"));

            let outcome: String = tester.run("help aliases bar")?;
            assert_contains("alias_line1", &outcome);
            assert_contains("alias_line2", &outcome);

            let outcome: String =
                tester.run("help aliases | where name == bar | get 0.description")?;
            assert_contains("alias_line1", &outcome);
            assert!(!outcome.contains("alias_line2"));
            Ok(())
        },
    )
}

#[test]
fn help_modules_main_1() -> Result {
    let mut tester = test();
    let () = tester.run(
        "module spam {
            export def main [] { 'foo' };
        }",
    )?;
    let outcome: String = tester.run("help spam")?;
    assert_contains("  spam", &outcome);
    Ok(())
}

#[test]
fn help_modules_main_2() -> Result {
    let mut tester = test();
    let () = tester.run(
        "module spam {
            export def main [] { 'foo' };
        }",
    )?;
    tester
        .run("help modules | where name == spam | get 0.commands.0.name")
        .expect_value_eq("spam")
}

#[test]
fn help_shows_module_qualified_usage() -> Result {
    let mut tester = test();
    let () = tester.run("module spam { export def prefix [p:string] { $p } }")?;
    let () = tester.run("use spam")?;
    let outcome: String = tester.run("help spam prefix")?;

    // Usage line should show the module-qualified command name
    assert_contains("> spam prefix", &outcome);
    Ok(())
}

#[test]
fn help_commands_shows_overlay_name_for_module_decls() -> Result {
    Playground::setup("help_overlay_name", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "spam.nu",
            "
                # exported function
                export def prefix [prefix: string] { $prefix }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        tester
            .run(r#"help commands | where name == "spam prefix" | length"#)
            .expect_value_eq(1)
    })
}

// Regression tests for #11657: an alias reuses the signature of the command it wraps. An alias of
// a subcommand is listed under the name it's called by, says what it runs, and sorts right after
// the subcommand it wraps.
#[rstest]
#[case::custom_command(
    r#"def foo [] {}; def "foo bar" [] {}; alias fb = foo bar; help foo | ansi strip"#,
    "foo bar (custom)",
    "fb (alias) - Alias for `foo bar`"
)]
#[case::builtin_command(
    "alias st = str trim; help str | ansi strip",
    "str trim - ",
    "st (alias) - Alias for `str trim`"
)]
#[case::module_alias(
    r#"
        module spam {
            export def foo [] {}
            export def "foo bar" [] {}
            export alias fb = foo bar
        }
        use spam
        help spam foo | ansi strip
    "#,
    "foo bar (custom)",
    "spam fb (alias) - Alias for `foo bar`"
)]
// A doc comment replaces the alias's description, but the list still says what the alias runs.
#[case::doc_comment(
    "
        # trim both ends
        alias st = str trim
        help str | ansi strip
    ",
    "str trim - ",
    "st (alias) - Alias for `str trim`"
)]
// The list has one line per entry, so an expansion written over several lines is joined.
#[case::multiline_expansion(
    r#"
        def foo [] {}
        def "foo bar" [x] {}
        alias fm = foo bar (
            'a'
        )
        help foo | ansi strip
    "#,
    "foo bar (custom)",
    "fm (alias) - Alias for `foo bar ( 'a' )`"
)]
// With color, the expansion is highlighted as a whole, so its own backticks aren't left over.
#[case::backticks_in_expansion(
    r#"
        $env.config.use_ansi_coloring = true
        def foo [] {}
        def "foo bar" [x] {}
        alias fq = foo bar `x y`
        help foo | ansi strip
    "#,
    "foo bar (custom)",
    "fq (alias) - Alias for foo bar `x y`"
)]
fn help_subcommands_list_alias_after_wrapped_subcommand(
    #[case] code: &str,
    #[case] wrapped: &str,
    #[case] alias_line: &str,
) -> Result {
    let outcome: String = test().run(code)?;
    let lines: Vec<&str> = outcome.lines().map(str::trim).collect();
    let wrapped_at = lines
        .iter()
        .position(|line| line.starts_with(wrapped))
        .unwrap_or_else(|| panic!("no `{wrapped}` line in:\n{outcome}"));
    assert_eq!(
        lines.get(wrapped_at + 1),
        Some(&alias_line),
        "got:\n{outcome}"
    );
    Ok(())
}

// A hidden subcommand can't be called, so it isn't listed.
#[rstest]
#[case::alias(
    r#"def foo [] {}; def "foo bar" [] {}; alias fb = foo bar; hide fb; help foo | ansi strip"#,
    "fb ("
)]
#[case::custom_command(
    r#"def foo [] {}; def "foo bar" [] {}; def "foo qux" [] {}; hide "foo qux"; help foo | ansi strip"#,
    "foo qux ("
)]
fn help_subcommands_omit_hidden_subcommand(#[case] code: &str, #[case] hidden: &str) -> Result {
    let outcome: String = test().run(code)?;
    assert_contains("foo bar (custom)", &outcome);
    assert_contains_not(hidden, &outcome);
    Ok(())
}

#[rstest]
#[case::top_level(
    r#"
        def foo [] {}
        def "foo bar" [] {}
        alias "foo baz" = foo bar
        help foo | ansi strip
    "#
)]
// Module subcommands are listed under their bare names (`foo bar`, not `spam foo bar`); that is
// the module-scoping TODO in `get_command_documentation` (#11447), not something this test pins.
#[case::module_prefixed(
    r#"
        module spam {
            export def foo [] {}
            export def "foo bar" [] {}
            export alias "foo baz" = foo bar
        }
        use spam
        help spam foo | ansi strip
    "#
)]
fn help_subcommands_list_alias_under_its_own_name(#[case] code: &str) -> Result {
    let outcome: String = test().run(code)?;
    let lines: Vec<&str> = outcome.lines().map(str::trim).collect();
    assert_eq!(
        lines
            .iter()
            .skip_while(|line| !line.starts_with("foo bar (custom)"))
            .take(2)
            .copied()
            .collect::<Vec<_>>(),
        [
            "foo bar (custom) -",
            "foo baz (alias) - Alias for `foo bar`"
        ],
        "got:\n{outcome}"
    );
    Ok(())
}

#[test]
#[exp(NATIVE_CLIP)]
fn clip_command_shows_module_qualified_decls_after_use() -> Result {
    // Ensure running `clip` shows module-qualified subcommands like `clip prefix` after `use std/clip`.
    let outcome: String = test().run("use std/clip; clip")?;
    assert_contains("clip prefix", &outcome);
    Ok(())
}
#[test]
fn help_module_lists_exported_type_aliases() -> Result {
    let code = "
        module geo {
            type Private = int
            export type Point = int
            export type Id = int
        }
        help modules geo
    ";

    let outcome: String = test().run(code)?;
    assert_contains("Exported type aliases:\n  Id, Point", &outcome);
    assert_contains_not("Private", &outcome);
    Ok(())
}

#[test]
fn nothing_type_annotation() -> Result {
    let mut tester = test();
    let () = tester.run("def foo []: nothing -> nothing {}")?;
    tester
        .run("help commands | where name == foo | get input_output.0.output.0")
        .expect_value_eq("nothing")
}
