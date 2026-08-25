use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn module_private_import_decl(playground: Playground) -> Result {
    playground.file(
        "main.nu",
        indoc::indoc! {"
            use spam.nu foo-helper

            export def foo [] { foo-helper }
        "},
    )?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            def get-foo [] { "foo" }
            export def foo-helper [] { get-foo }
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_private_import_alias(playground: Playground) -> Result {
    playground.file(
        "main.nu",
        indoc::indoc! {"
            use spam.nu foo-helper

            export def foo [] { foo-helper }
        "},
    )?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            export alias foo-helper = echo "foo"
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_private_import_decl_not_public(playground: Playground) -> Result {
    playground.file("main.nu", "use spam.nu foo-helper")?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            def get-foo [] { "foo" }
            export def foo-helper [] { get-foo }
        "#},
    )?;

    test()
        .cwd(playground.path())
        .run("use main.nu foo")
        .expect_error_code_eq("nu::parser::export_not_found")
}

#[test]
fn module_public_import_decl(playground: Playground) -> Result {
    playground.file("main.nu", "export use spam.nu foo")?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            def foo-helper [] { "foo" }
            export def foo [] { foo-helper }
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_public_import_alias(playground: Playground) -> Result {
    playground.file("main.nu", "export use spam.nu foo")?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            export alias foo = echo "foo"
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_public_import_decl_with_stored_where_condition(playground: Playground) -> Result {
    playground.file("main.nu", "export use mod.nu helper")?;

    playground.file(
        "mod.nu",
        indoc::indoc! {r#"
            export def helper [] {
                let cond = {|x| true }
                [{a: 1}] | where $cond
            }
            
            export def main [] { "ok" }
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu helper")?;
    tester
        .run("helper | to nuon --raw")
        .expect_value_eq("[[a];[1]]")
}

#[test]
fn module_nested_imports(playground: Playground) -> Result {
    playground.file("main.nu", "export use spam.nu [ foo bar ]")?;
    playground.file("spam.nu", "export use spam2.nu [ foo bar ]")?;
    playground.file("spam2.nu", "export use spam3.nu [ foo bar ]")?;
    playground.file(
        "spam3.nu",
        indoc::indoc! {r#"
            export def foo [] { "foo" }
            export alias bar = echo "bar"
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu bar")?;
    tester.run("bar").expect_value_eq("bar")
}

#[test]
fn module_nested_imports_in_dirs(playground: Playground) -> Result {
    playground.file("main.nu", "export use spam/spam.nu [ foo bar ]")?;
    playground.at("spam", |spam| {
        spam.file("spam.nu", "export use spam2/spam2.nu [ foo bar ]")?;
        spam.file("spam2/spam2.nu", "export use ../spam3/spam3.nu [ foo bar ]")?;
        spam.file(
            "spam3/spam3.nu",
            indoc::indoc! {r#"
                export def foo [] { "foo" }
                export alias bar = echo "bar"
            "#},
        )
    })?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu bar")?;
    tester.run("bar").expect_value_eq("bar")
}

#[test]
fn module_public_import_decl_prefixed(playground: Playground) -> Result {
    playground.file("main.nu", "export use spam.nu")?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            def foo-helper [] { "foo" }
            export def foo [] { foo-helper }
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu")?;
    tester.run("main spam foo").expect_value_eq("foo")
}

#[test]
fn module_nested_imports_in_dirs_prefixed(playground: Playground) -> Result {
    playground.file(
        "main.nu",
        r#"export use spam/spam.nu [ "spam2 foo" "spam2 spam3 bar" ]"#,
    )?;
    playground.at("spam", |spam| {
        spam.file("spam.nu", "export use spam2/spam2.nu")?;
        spam.file(
            "spam2/spam2.nu",
            indoc::indoc! {"
                export use ../spam3/spam3.nu
                export use ../spam3/spam3.nu foo
            "},
        )?;
        spam.file(
            "spam3/spam3.nu",
            indoc::indoc! {r#"
                export def foo [] { "foo" }
                export alias bar = echo "bar"
            "#},
        )
    })?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use main.nu")?;
    tester.run("main spam2 foo").expect_value_eq("foo")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(r#"use main.nu "spam2 spam3 bar""#)?;
    tester.run("spam2 spam3 bar").expect_value_eq("bar")
}

#[test]
fn module_import_env_1(playground: Playground) -> Result {
    playground.file(
        "main.nu",
        indoc::indoc! {"
            export-env { source-env spam.nu }

            export def foo [] { $env.FOO_HELPER }
        "},
    )?;
    playground.file(
        "spam.nu",
        indoc::indoc! {r#"
            export-env { $env.FOO_HELPER = "foo" }
        "#},
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("source-env main.nu")?;
    let () = tester.run("use main.nu foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_import_env_2(playground: Playground) -> Result {
    playground.file("main.nu", "export-env { source-env spam.nu }")?;
    playground.file("spam.nu", r#"export-env { $env.FOO = "foo" }"#)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("source-env main.nu")?;
    tester.run("$env.FOO").expect_value_eq("foo")
}

#[test]
fn module_cyclical_imports_0(playground: Playground) -> Result {
    playground.file("spam.nu", "use eggs.nu")?;

    test()
        .cwd(playground.path())
        .run("module eggs { use spam.nu }")
        .expect_error_code_eq("nu::parser::module_not_found")
}

#[test]
fn module_cyclical_imports_1(playground: Playground) -> Result {
    playground.file("spam.nu", "use spam.nu")?;

    test()
        .cwd(playground.path())
        .run("use spam.nu")
        .expect_error_code_eq("nu::parser::circular_import")
}

#[test]
fn module_cyclical_imports_2(playground: Playground) -> Result {
    playground.file("spam.nu", "use eggs.nu")?;
    playground.file("eggs.nu", "use spam.nu")?;

    test()
        .cwd(playground.path())
        .run("use spam.nu")
        .expect_error_code_eq("nu::parser::circular_import")
}

#[test]
fn module_cyclical_imports_3(playground: Playground) -> Result {
    playground.file("spam.nu", "use eggs.nu")?;
    playground.file("eggs.nu", "use bacon.nu")?;
    playground.file("bacon.nu", "use spam.nu")?;

    test()
        .cwd(playground.path())
        .run("use spam.nu")
        .expect_error_code_eq("nu::parser::circular_import")
}

#[test]
fn module_import_const_file(playground: Playground) -> Result {
    playground.file("spam.nu", r#"export def foo [] { "foo" }"#)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("const file = 'spam.nu'")?;
    let () = tester.run("use $file foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_import_const_module_name(playground: Playground) -> Result {
    playground.file("spam.nu", r#"export def foo [] { "foo" }"#)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(r#"module spam { export def foo [] { "foo" } }"#)?;
    let () = tester.run("const mod = 'spam'")?;
    let () = tester.run("use $mod foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_valid_def_name() -> Result {
    test()
        .run(r#"module spam { def spam [] { "spam" } }"#)
        .expect_value_eq(())
}

#[test]
fn module_invalid_def_name() -> Result {
    test()
        .run(r#"module spam { export def spam [] { "spam" } }"#)
        .expect_error_code_eq("nu::parser::named_as_module")
}

#[test]
fn module_valid_alias_name_1() -> Result {
    test()
        .run(r#"module spam { alias spam = echo "spam" }"#)
        .expect_value_eq(())
}

#[test]
fn module_valid_alias_name_2() -> Result {
    test()
        .run(r#"module spam { alias main = echo "spam" }"#)
        .expect_value_eq(())
}

#[test]
fn module_invalid_alias_name() -> Result {
    test()
        .run(r#"module spam { export alias spam = echo "spam" }"#)
        .expect_error_code_eq("nu::parser::named_as_module")
}

#[test]
fn module_main_alias_not_allowed() -> Result {
    test()
        .run("module spam { export alias main = echo 'spam' }")
        .expect_error_code_eq("nu::parser::export_main_alias_not_allowed")
}

#[test]
fn module_valid_known_external_name() -> Result {
    test()
        .run("module spam { extern spam [] }")
        .expect_value_eq(())
}

#[test]
fn module_invalid_known_external_name() -> Result {
    test()
        .run("module spam { export extern spam [] }")
        .expect_error_code_eq("nu::parser::named_as_module")
}

#[test]
fn main_inside_module_is_main() -> Result {
    let mut tester = test();
    let () = tester.run(
        "
        module spam {
            export def main [] { 'foo' };
            export def foo [] { main }
        }
        ",
    )?;
    let () = tester.run("use spam foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn module_as_file() -> Result {
    let mut tester = test().cwd("tests/modules");
    let () = tester.run("module samples/spam.nu")?;
    let () = tester.run("use spam foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn export_module_as_file() -> Result {
    let mut tester = test().cwd("tests/modules");
    let () = tester.run("export module samples/spam.nu")?;
    let () = tester.run("use spam foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[test]
fn deep_import_patterns() -> Result {
    let module_decl = "
        module spam {
            export module eggs {
                export module beans {
                    export def foo [] { 'foo' };
                    export def bar [] { 'bar' }
                }; export use beans
            }; export use eggs
        }
    ";

    let mut tester = test();
    let () = tester.run(module_decl)?;
    let () = tester.run("use spam")?;
    tester.run("spam eggs beans foo").expect_value_eq("foo")?;

    let mut tester = test();
    let () = tester.run(module_decl)?;
    let () = tester.run("use spam eggs")?;
    tester.run("eggs beans foo").expect_value_eq("foo")?;

    let mut tester = test();
    let () = tester.run(module_decl)?;
    let () = tester.run("use spam eggs beans")?;
    tester.run("beans foo").expect_value_eq("foo")?;

    let mut tester = test();
    let () = tester.run(module_decl)?;
    let () = tester.run("use spam eggs beans foo")?;
    tester.run("foo").expect_value_eq("foo")
}

#[rstest]
fn deep_import_aliased_external_args(
    #[values(
        "use spam; spam eggs beans foo bar",
        "use spam eggs; eggs beans foo bar",
        "use spam eggs beans; beans foo bar",
        "use spam eggs beans foo; foo bar"
    )]
    input: &str,
) -> Result {
    let module_decl = "
        module spam {
            export module eggs {
                export module beans {
                    export alias foo = ^echo
                }; export use beans
            }; export use eggs
        }
    ";
    let mut tester = test().inherit_path();
    let () = tester.run(module_decl)?;
    tester.run(input).expect_value_eq("bar")
}

#[rstest]
#[case("spam", "spam")]
#[case("spam foo", "foo")]
#[case("spam bar", "bar")]
#[case("spam foo baz", "foobaz")]
#[case("spam bar baz", "barbaz")]
#[case("spam baz", "spambaz")]
#[case::deep("spam bacon", "bacon")]
#[case::deep("spam bacon foo", "bacon foo")]
#[case::deep("spam bacon beans", "beans")]
#[case::deep("spam bacon beans foo", "beans foo")]
fn module_dir(#[case] code: &str, #[case] expected: impl IntoValue) -> Result {
    let mut tester = test().cwd("tests/modules");
    let () = tester.run("use samples/spam")?;
    tester.run(code).expect_value_eq(expected)
}

#[test]
fn module_dir_import_twice_no_panic() -> Result {
    let import = "use samples/spam";
    let mut tester = test().cwd("tests/modules");
    let () = tester.run(import)?;
    let () = tester.run(import)?;
    tester.run("spam").expect_value_eq("spam")
}

#[test]
fn module_dir_missing_mod_nu() -> Result {
    test()
        .cwd("tests/modules")
        .run("use samples/missing_mod_nu")
        .expect_error_code_eq("nu::parser::module_missing_mod_nu_file")
}

#[test]
fn allowed_local_module() -> Result {
    test()
        .run("module spam { module spam {} }")
        .expect_value_eq(())
}

#[test]
fn not_allowed_submodule() -> Result {
    test()
        .run("module spam { export module spam {} }")
        .expect_error_code_eq("nu::parser::named_as_module")
}

#[test]
fn module_self_name() -> Result {
    let mut tester = test();
    let () = tester.run("module spam { export module mod { export def main [] { 'spam' } } }")?;
    let () = tester.run("use spam")?;
    tester.run("spam").expect_value_eq("spam")
}

#[test]
fn module_self_name_main_not_allowed() -> Result {
    test()
        .run(
            "
            module spam {
                export def main [] { 'main spam' };

                export module mod {
                    export def main [] { 'mod spam' }
                }
            }
            ",
        )
        .expect_error_code_eq("nu::parser::module_double_main")?;

    test()
        .run(
            "
            module spam {
                export module mod {
                    export def main [] { 'mod spam' }
                };

                export def main [] { 'main spam' }
            }
            ",
        )
        .expect_error_code_eq("nu::parser::module_double_main")
}

#[test]
fn module_main_not_found() -> Result {
    let mut tester = test();
    let () = tester.run("module spam {}")?;
    tester
        .run("use spam main")
        .expect_error_code_eq("nu::parser::export_not_found")?;

    let mut tester = test();
    let () = tester.run("module spam {}")?;
    tester
        .run("use spam [ main ]")
        .expect_error_code_eq("nu::parser::export_not_found")
}

#[test]
fn nested_list_export_works() -> Result {
    let module = "
        module spam {
            export module eggs {
                export def bacon [] { 'bacon' }
            }

            export def sausage [] { 'sausage' }
        }
    ";

    let mut tester = test();
    let () = tester.run(module)?;
    let () = tester.run("use spam [sausage eggs]")?;
    tester.run("eggs bacon").expect_value_eq("bacon")
}

#[rstest]
#[case(
    &[
        "use voice.nu",
        r#""export def cat [] { 'woem' }" | save -f animals.nu"#,
        "use voice.nu",
    ],
    "voice animals cat",
    "woem"
)]
#[case(
    &[
        "use voice.nu",
        r#""export def cat [] { 'meow' }" | save -f animals.nu"#,
        // should also verify something unchanged if `use voice`.
        "use voice",
    ],
    "voice animals cat",
    "meow"
)]
#[case(
    &[
        "use voice.nu animals cat",
        r#""export def cat [] { 'woem' }" | save -f animals.nu"#,
        "use voice.nu animals cat",
    ],
    "cat",
    "woem"
)]
fn reload_submodules(
    #[ignore] playground: Playground,
    #[case] setup: &[&str],
    #[case] code: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    playground.file("voice.nu", "export module animals.nu; export use animals")?;
    playground.file("animals.nu", "export def cat [] { 'meow' }")?;

    let mut tester = test().cwd(playground.path());
    for line in setup {
        let () = tester.run(line)?;
    }
    tester.run(code).expect_value_eq(expected)
}

#[test]
fn use_submodules(playground: Playground) -> Result {
    playground.file("voice.nu", "export use animals.nu")?;
    playground.file("animals.nu", "export def cat [] { 'meow' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester
        .run("(voice animals cat) == 'woem'")
        .expect_value_eq(true)?;

    // should also verify something unchanged if `use voice`.
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'meow' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice")?;
    tester
        .run("(voice animals cat) == 'woem'")
        .expect_value_eq(true)?;

    // also verify something is changed when using members.
    playground.file("voice.nu", "export use animals.nu cat")?;
    playground.file("animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester.run("(voice cat) == 'woem'").expect_value_eq(true)?;

    playground.file("voice.nu", "export use animals.nu *")?;

    playground.file("animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester.run("(voice cat) == 'woem'").expect_value_eq(true)?;

    playground.file("voice.nu", "export use animals.nu [cat]")?;

    playground.file("animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester.run("(voice cat) == 'woem'").expect_value_eq(true)
}

#[test]
fn use_nested_submodules(playground: Playground) -> Result {
    playground.file("voice.nu", "export use animals.nu")?;
    playground.file("animals.nu", "export use nested_animals.nu")?;
    playground.file("nested_animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f nested_animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester
        .run("(voice animals nested_animals cat) == 'woem'")
        .expect_value_eq(true)?;

    playground.file("voice.nu", "export use animals.nu")?;

    playground.file("animals.nu", "export use nested_animals.nu cat")?;

    playground.file("nested_animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f nested_animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester
        .run("(voice animals cat) == 'woem'")
        .expect_value_eq(true)?;

    playground.file("animals.nu", "export use nested_animals.nu cat")?;

    playground.file("nested_animals.nu", "export def cat [] { 'meow' }")?;
    let mut tester = test().cwd(playground.path());
    let () = tester.run("module voice { export module animals.nu }")?;
    let () = tester.run("use voice")?;
    let () = tester.run(r#""export def cat [] { 'woem' }" | save -f nested_animals.nu"#)?;
    let () = tester.run("use voice.nu")?;
    tester
        .run("(voice animals cat) == 'woem'")
        .expect_value_eq(true)
}
