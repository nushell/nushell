use nu_experimental::NATIVE_CLIP;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

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

#[test]
#[exp(NATIVE_CLIP)]
fn clip_command_shows_module_qualified_decls_after_use() -> Result {
    // Ensure running `clip` shows module-qualified subcommands like `clip prefix` after `use std/clip`.
    let outcome: String = test().run("use std/clip; clip")?;
    assert_contains("clip prefix", &outcome);
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
