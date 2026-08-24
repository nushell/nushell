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
    playground.dir("spam")?;
    playground.dir("spam/spam2")?;
    playground.dir("spam/spam3")?;
    playground.file("main.nu", "export use spam/spam.nu [ foo bar ]")?;
    playground.file("spam/spam.nu", "export use spam2/spam2.nu [ foo bar ]")?;
    playground.file(
        "spam/spam2/spam2.nu",
        "export use ../spam3/spam3.nu [ foo bar ]",
    )?;
    playground.file(
        "spam/spam3/spam3.nu",
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
    playground.dir("spam")?;
    playground.dir("spam/spam2")?;
    playground.dir("spam/spam3")?;
    playground.file(
        "main.nu",
        r#"export use spam/spam.nu [ "spam2 foo" "spam2 spam3 bar" ]"#,
    )?;
    playground.file("spam/spam.nu", "export use spam2/spam2.nu")?;
    playground.file(
        "spam/spam2/spam2.nu",
        indoc::indoc! {"
                    export use ../spam3/spam3.nu
                    export use ../spam3/spam3.nu foo
                "},
    )?;
    playground.file(
        "spam/spam3/spam3.nu",
        indoc::indoc! {r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#},
    )?;

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
fn module_dir_name_keeps_dots(playground: Playground) -> Result {
    playground.dir("spam.eggs")?;
    playground.file("spam.eggs/mod.nu", r#"export def foo [] { "foo" }"#)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.eggs")?;
    tester.run("spam.eggs foo").expect_value_eq("foo")?;
    tester
        .run("scope modules | where name == 'spam.eggs' | length")
        .expect_value_eq(1)
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
    #[case] setup: &[&str],
    #[case] code: &str,
    #[case] expected: impl IntoValue,
    #[ignore] playground: Playground,
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

    // `use voice` by name picks up the change too.
    let mut tester = test().cwd(playground.path());
    let () = tester.run("use voice.nu")?;
    let () = tester.run(r#""export def cat [] { 'meow' }" | save -f animals.nu"#)?;
    let () = tester.run("use voice")?;
    tester
        .run("(voice animals cat) == 'meow'")
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

#[rstest]
// `shared.nu` is reached through both `a.nu` and `b.nu`: changing it must reload both.
#[case(r#""export def s [] { 'woem' }" | save -f shared.nu"#, ["woem", "woem"])]
// The walk finds `a.nu` and `shared.nu` unchanged first; marking them as seen must not stop it
// from going on to find the change in `b.nu`.
#[case(r#""use shared.nu; export def y [] { 'woem' }" | save -f b.nu"#, ["meow", "woem"])]
fn reload_module_reached_through_several_imports(
    #[case] change: &str,
    #[case] expected: [&str; 2],
    #[ignore] playground: Playground,
) -> Result {
    playground.file("lib.nu", "export use a.nu; export use b.nu")?;
    playground.file("a.nu", "use shared.nu; export def x [] { shared s }")?;
    playground.file("b.nu", "use shared.nu; export def y [] { shared s }")?;
    playground.file("shared.nu", "export def s [] { 'meow' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use lib.nu")?;
    let () = tester.run(change)?;
    let () = tester.run("use lib.nu")?;
    tester
        .run("[(lib a x) (lib b y)]")
        .expect_value_eq(expected.to_vec())
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

// `use <name>` reads the module's file again once it has changed, like `use <path>` does.
#[rstest]
#[case::prefixed("use_name_prefixed", "use spam.nu", "use spam", "spam foo")]
#[case::glob("use_name_glob", "use spam.nu *", "use spam *", "foo")]
#[case::member("use_name_member", "use spam.nu foo", "use spam foo", "foo")]
#[case::member_list("use_name_list", "use spam.nu [foo]", "use spam [foo]", "foo")]
fn use_by_name_reads_a_changed_file(
    #[case] _topic: &str,
    #[case] first: &str,
    #[case] again: &str,
    #[case] call: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file("spam.nu", "export def foo [] { 'old' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(first)?;
    let () = tester.run(r#""export def foo [] { 'new' }" | save -f spam.nu"#)?;
    // Nothing changes until the next `use`.
    tester.run(call).expect_value_eq("old")?;
    let () = tester.run(again)?;
    tester.run(call).expect_value_eq("new")
}

#[rstest]
#[case::mod_nu("use_name_dir_mod", "pkg/mod.nu")]
#[case::imported("use_name_dir_import", "pkg/sub.nu")]
fn use_by_name_reads_a_changed_directory_module(
    #[case] _topic: &str,
    #[case] edited: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.dir("pkg")?;
    playground.file("pkg/mod.nu", "export use sub.nu *")?;
    playground.file("pkg/sub.nu", "export def foo [] { 'old' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use pkg")?;
    tester.run("pkg foo").expect_value_eq("old")?;
    let () = tester.run_with_data(format!("save -f {edited}"), "export def foo [] { 'new' }")?;
    let () = tester.run("use pkg")?;
    tester.run("pkg foo").expect_value_eq("new")
}

#[test]
fn use_by_name_runs_a_changed_export_env(playground: Playground) -> Result {
    playground.file("spam.nu", "export-env { $env.SPAM = 'old' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu")?;
    let () = tester.run(r#""export-env { $env.SPAM = 'new' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam")?;
    tester.run("$env.SPAM").expect_value_eq("new")
}

// A module written at the prompt has no file to read, even if one with its name sits in the cwd.
#[test]
fn use_by_name_keeps_an_inline_module(playground: Playground) -> Result {
    playground.file("spam.nu", "export def foo [] { 'file' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("module spam { export def foo [] { 'inline' } }")?;
    let () = tester.run("use spam")?;
    tester.run("spam foo").expect_value_eq("inline")
}

#[test]
fn use_by_name_keeps_the_module_when_its_file_is_gone(playground: Playground) -> Result {
    playground.file("spam.nu", "export def foo [] { 'old' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu")?;
    std::fs::remove_file(playground.path().join("spam.nu"))?;
    let () = tester.run("use spam")?;
    tester.run("spam foo").expect_value_eq("old")
}

// Reloading drops what the new version no longer exports, however the module was imported.
#[rstest]
#[case::path_prefixed(
    "deleted_path_prefixed",
    "use spam.nu",
    "use spam.nu",
    "spam bar",
    "spam foo"
)]
#[case::path_glob("deleted_path_glob", "use spam.nu *", "use spam.nu *", "bar", "foo")]
#[case::name_prefixed(
    "deleted_name_prefixed",
    "use spam.nu",
    "use spam",
    "spam bar",
    "spam foo"
)]
#[case::name_glob("deleted_name_glob", "use spam.nu *", "use spam *", "bar", "foo")]
#[case::alias_prefixed(
    "deleted_alias_prefixed",
    "use spam.nu",
    "use spam.nu",
    "spam baz",
    "spam foo"
)]
#[case::alias_glob("deleted_alias_glob", "use spam.nu *", "use spam.nu *", "baz", "foo")]
#[case::main_prefixed(
    "deleted_main_prefixed",
    "use spam.nu",
    "use spam.nu",
    "spam",
    "spam foo"
)]
#[case::main_glob("deleted_main_glob", "use spam.nu *", "use spam.nu *", "spam", "foo")]
#[case::prefixed_then_glob("deleted_mixed", "use spam.nu", "use spam.nu *", "spam bar", "foo")]
#[case::submodule(
    "deleted_submodule",
    "use spam.nu *",
    "use spam.nu *",
    "sub cmd",
    "foo"
)]
fn deleted_definition_goes_away_on_reload(
    #[case] _topic: &str,
    #[case] first: &str,
    #[case] again: &str,
    #[case] deleted: &str,
    #[case] kept: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file(
        "spam.nu",
        "
            export module sub { export def cmd [] { 'cmd' } }
            export def main [] { 'main' }
            export def foo [] { 'old' }
            export def bar [] { 'bar' }
            export alias baz = echo 'baz'
            ",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(first)?;
    let _: Value = tester.run(deleted)?;
    let () = tester.run(r#""export def foo [] { 'new' }" | save -f spam.nu"#)?;
    let () = tester.run(again)?;
    tester.run(kept).expect_value_eq("new")?;
    tester
        .run(deleted)
        .expect_error_code_eq("nu::shell::external_command")
}

#[test]
fn deleted_type_alias_goes_away_on_reload(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export type Animal = string; export def foo [] { 'foo' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run("let a: Animal = 'cat'")?;
    let () = tester.run(r#""export def foo [] { 'foo' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam.nu *")?;
    tester
        .run("let b: Animal = 'cat'")
        .expect_error_code_eq("nu::parser::unknown_type")
}

#[test]
fn reload_keeps_a_command_redefined_since_the_import(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'bar' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run("def bar [] { 'mine' }")?;
    let () = tester.run(r#""export def foo [] { 'foo' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam.nu *")?;
    tester.run("bar").expect_value_eq("mine")
}

// Dropping a re-export doesn't undo importing the same command from its own module.
#[rstest]
#[case::direct(
    "dropped_reexport_direct",
    "export use b.nu [foo bar]",
    "export use b.nu [foo]"
)]
#[case::through_another_module(
    "dropped_reexport_chain",
    "export use c.nu [foo bar]",
    "export use c.nu [foo]"
)]
fn reload_keeps_a_command_imported_from_its_own_module(
    #[case] _topic: &str,
    #[case] first: &str,
    #[case] edited: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file(
        "b.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'bar' }",
    )?;
    playground.file("c.nu", "export use b.nu [foo bar]")?;
    playground.file("a.nu", first)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use b.nu bar")?;
    let () = tester.run("use a.nu *")?;
    let () = tester.run_with_data("save -f a.nu", edited)?;
    let () = tester.run("use a *")?;
    tester.run("bar").expect_value_eq("bar")
}

#[test]
fn reload_hides_a_dropped_reexport_you_only_got_through_the_module(
    playground: Playground,
) -> Result {
    playground.file(
        "b.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'bar' }",
    )?;
    playground.file("a.nu", "export use b.nu [foo bar]")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use a.nu *")?;
    let () = tester.run_with_data("save -f a.nu", "export use b.nu [foo]")?;
    let () = tester.run("use a *")?;
    tester.run("foo").expect_value_eq("foo")?;
    tester
        .run("bar")
        .expect_error_code_eq("nu::shell::external_command")
}

// `use a.nu *` binds `b` when `a` has `export use b.nu`. That doesn't count as importing from `b`.
#[test]
fn reload_hides_a_deleted_command_of_a_module_the_glob_bound(playground: Playground) -> Result {
    playground.file(
        "b.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'bar' }",
    )?;
    playground.file("a.nu", "export use b.nu")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use a.nu *")?;
    let () = tester.run_with_data("save -f b.nu", "export def foo [] { 'new' }")?;
    let () = tester.run("use a *")?;
    tester.run("b foo").expect_value_eq("new")?;
    tester
        .run("b bar")
        .expect_error_code_eq("nu::shell::external_command")
}

// A reload moves every name still showing the old version, not only the ones this `use` named.
#[rstest]
#[case::bare_after_prefixed(
    "rebind_bare_after_prefixed",
    &["use spam.nu *"],
    "use spam",
    "foo"
)]
#[case::member_after_another_member(
    "rebind_member",
    &["use spam.nu foo", "use spam.nu bar"],
    "use spam foo",
    "bar"
)]
#[case::prefixed_after_member(
    "rebind_prefixed",
    &["use spam.nu"],
    "use spam foo",
    "spam bar"
)]
#[case::by_path("rebind_by_path", &["use spam.nu foo", "use spam.nu bar"], "use spam.nu foo", "bar")]
fn reload_moves_kept_names_to_the_new_version(
    #[case] _topic: &str,
    #[case] first: &[&str],
    #[case] again: &str,
    #[case] call: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'old' }; export def bar [] { 'old' }",
    )?;

    let mut tester = test().cwd(playground.path());
    for command in first {
        let () = tester.run(*command)?;
    }
    let () = tester.run_with_data(
        "save -f spam.nu",
        "export def foo [] { 'new' }; export def bar [] { 'new' }",
    )?;
    let () = tester.run(again)?;
    tester.run(call).expect_value_eq("new")
}

#[test]
fn reload_moves_kept_constants_and_submodules_to_the_new_version(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export const X = 'old'; export module sub { export def x [] { 'old' } }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data(
        "save -f spam.nu",
        "export const X = 'new'; export module sub { export def x [] { 'new' } }",
    )?;
    let () = tester.run("use spam")?;
    tester.run("$X").expect_value_eq("new")?;
    let () = tester.run("use sub")?;
    tester.run("sub x").expect_value_eq("new")
}

#[rstest]
#[case::glob("deleted_const_glob", "use spam *")]
#[case::prefixed("deleted_const_prefixed", "use spam")]
#[case::by_path("deleted_const_path", "use spam.nu *")]
fn deleted_constant_goes_away_on_reload(
    #[case] _topic: &str,
    #[case] again: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file("spam.nu", "export const X = 'old'; export const Y = 'old'")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export const Y = 'new'")?;
    let () = tester.run(again)?;
    tester.run("$Y").expect_value_eq("new")?;
    tester
        .run("$X")
        .expect_error_code_eq("nu::parser::variable_not_found")
}

#[test]
fn reload_keeps_a_constant_redefined_since_the_import(playground: Playground) -> Result {
    playground.file("spam.nu", "export const X = 'old'; export const Y = 'old'")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run("const X = 'mine'")?;
    let () = tester.run_with_data("save -f spam.nu", "export const Y = 'new'")?;
    let () = tester.run("use spam *")?;
    tester.run("$X").expect_value_eq("mine")
}

#[test]
fn reload_keeps_a_constant_imported_from_its_own_module(playground: Playground) -> Result {
    playground.file("b.nu", "export const X = 'x'; export const Y = 'y'")?;
    playground.file("a.nu", "export use b.nu [X Y]")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use b.nu X")?;
    let () = tester.run("use a.nu *")?;
    let () = tester.run_with_data("save -f a.nu", "export use b.nu [Y]")?;
    let () = tester.run("use a *")?;
    tester.run("$X").expect_value_eq("x")
}

#[test]
fn reload_keeps_a_type_alias_imported_from_its_own_module(playground: Playground) -> Result {
    playground.file(
        "b.nu",
        "export type Animal = string; export type Size = int",
    )?;
    playground.file("a.nu", "export use b.nu [Animal Size]")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use b.nu Animal")?;
    let () = tester.run("use a.nu *")?;
    let () = tester.run_with_data("save -f a.nu", "export use b.nu [Size]")?;
    let () = tester.run("use a *")?;
    let () = tester.run("let a: Size = 1")?;
    let () = tester.run("let b: Animal = 'cat'")?;
    Ok(())
}

#[test]
fn reload_moves_a_kept_type_alias_to_the_new_version(playground: Playground) -> Result {
    playground.file("spam.nu", "export type Animal = string")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export type Animal = int")?;
    let () = tester.run("use spam")?;
    let () = tester.run("let a: Animal = 1")?;
    tester
        .run("let b: Animal = 'cat'")
        .expect_error_code_eq("nu::parser::type_mismatch")
}

const SPAM_WITH_SUBMODULE: &str =
    "export module sub { export def x [] { 'old' } }; export def foo [] { 'old' }";

#[rstest]
#[case::glob("deleted_sub_glob", "use spam.nu *", "use spam *")]
#[case::prefixed_reload("deleted_sub_prefixed", "use spam.nu *", "use spam")]
#[case::by_path("deleted_sub_path", "use spam.nu *", "use spam.nu *")]
#[case::imported_by_name("deleted_sub_named", "use spam.nu sub", "use spam")]
fn deleted_submodule_goes_away_on_reload(
    #[case] _topic: &str,
    #[case] first: &str,
    #[case] again: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file("spam.nu", SPAM_WITH_SUBMODULE)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(first)?;
    let () = tester.run("use sub")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    let () = tester.run(again)?;
    tester
        .run("use sub")
        .expect_error_code_eq("nu::parser::module_not_found")?;
    tester
        .run("scope modules | where name == sub | is-empty")
        .expect_value_eq(true)
}

#[test]
fn deleted_submodule_goes_away_in_the_source_unit_of_the_reload(playground: Playground) -> Result {
    playground.file("spam.nu", SPAM_WITH_SUBMODULE)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    tester
        .run("use spam *; use sub")
        .expect_error_code_eq("nu::parser::module_not_found")
}

// The file a deleted submodule came from can still be there, and importing it binds the name.
#[rstest]
#[case::same_source_unit("sub_again_same_unit", &["use spam *; use sub.nu; use sub"])]
#[case::later_source_unit("sub_again_later_unit", &["use spam *", "use sub.nu"])]
fn importing_a_deleted_submodule_again_brings_it_back(
    #[case] _topic: &str,
    #[case] again: &[&str],
    #[ignore] playground: Playground,
) -> Result {
    playground.file("sub.nu", "export def x [] { 'sub' }")?;
    playground.file(
        "spam.nu",
        "export module sub.nu; export def foo [] { 'old' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    for command in again {
        let () = tester.run(*command)?;
    }
    let () = tester.run("use sub")?;
    tester.run("sub x").expect_value_eq("sub")
}

#[test]
fn importing_a_deleted_submodule_in_a_block_brings_it_back_there(playground: Playground) -> Result {
    playground.file("sub.nu", "export def x [] { 'sub' }")?;
    playground.file(
        "spam.nu",
        "export module sub.nu; export def foo [] { 'old' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    tester
        .run("use spam *; do { use sub.nu; use sub; sub x }")
        .expect_value_eq("sub")?;
    tester
        .run("use sub")
        .expect_error_code_eq("nu::parser::module_not_found")
}

#[test]
fn submodule_added_back_after_a_reload_is_found(playground: Playground) -> Result {
    let with_sub = "export module sub.nu; export def foo [] { 'old' }";
    playground.file("sub.nu", "export def x [] { 'sub' }")?;
    playground.file("spam.nu", with_sub)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    let () = tester.run("use spam *")?;
    let () = tester.run_with_data("save -f spam.nu", with_sub)?;
    let () = tester.run("use spam *")?;
    let () = tester.run("use sub")?;
    tester.run("sub x").expect_value_eq("sub")
}

#[test]
fn reload_keeps_your_own_module_with_a_dropped_name(playground: Playground) -> Result {
    playground.file("spam.nu", SPAM_WITH_SUBMODULE)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run("module sub { export def x [] { 'mine' } }")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    let () = tester.run("use spam *")?;
    let () = tester.run("use sub")?;
    tester.run("sub x").expect_value_eq("mine")
}

#[test]
fn reload_in_a_block_hides_a_dropped_submodule_only_there(playground: Playground) -> Result {
    playground.file("spam.nu", SPAM_WITH_SUBMODULE)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    tester
        .run("do { use spam *; use sub }")
        .expect_error_code_eq("nu::parser::module_not_found")?;

    let () = tester.run("do { use spam * }")?;
    let () = tester.run("use sub")?;
    tester.run("sub x").expect_value_eq("old")?;

    let () = tester.run("use spam *")?;
    tester
        .run("use sub")
        .expect_error_code_eq("nu::parser::module_not_found")
}

// Moving names doesn't undo what you did to them since the import.
#[rstest]
#[case::your_own_definition("rebind_keeps_own", "def foo [] { 'mine' }", "foo", "mine")]
#[case::your_own_constant("rebind_keeps_const", "const X = 'mine'", "$X", "mine")]
fn reload_leaves_your_own_definitions_alone(
    #[case] _topic: &str,
    #[case] yours: &str,
    #[case] call: &str,
    #[case] expected: &str,
    #[ignore] playground: Playground,
) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'old' }; export const X = 'old'",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run(yours)?;
    let () = tester.run_with_data(
        "save -f spam.nu",
        "export def foo [] { 'new' }; export const X = 'new'",
    )?;
    let () = tester.run("use spam")?;
    tester.run(call).expect_value_eq(expected)
}

#[test]
fn reload_leaves_a_hidden_command_hidden(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'old' }; export def bar [] { 'old' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run("hide bar")?;
    let () = tester.run_with_data(
        "save -f spam.nu",
        "export def foo [] { 'new' }; export def bar [] { 'new' }",
    )?;
    let () = tester.run("use spam")?;
    tester.run("foo").expect_value_eq("new")?;
    tester
        .run("bar")
        .expect_error_code_eq("nu::shell::external_command")
}

// `hide <module>` reaches a bare name imported before the reload, because the reload moved it.
#[test]
fn hide_after_a_prefixed_reload_hides_earlier_bare_names(playground: Playground) -> Result {
    playground.file("spam.nu", "export def foo [] { 'old' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu *")?;
    let () = tester.run_with_data("save -f spam.nu", "export def foo [] { 'new' }")?;
    let () = tester.run("use spam")?;
    let () = tester.run("hide spam")?;
    tester
        .run("foo")
        .expect_error_code_eq("nu::shell::external_command")
}

// A bare module name isn't a path, so `FILE_PWD` has to come from the module's file. This one had
// no `export-env` to leak `FILE_PWD` into the environment the first time.
#[test]
fn use_by_name_reload_sets_file_pwd(playground: Playground) -> Result {
    playground.dir("mods")?;
    playground.file("mods/foo.nu", "export def v [] { 1 }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use mods/foo.nu")?;
    let () = tester.run_with_data(
        "save -f mods/foo.nu",
        "export-env { $env.WHERE = ($env.FILE_PWD | path basename) }",
    )?;
    let () = tester.run("use foo")?;
    tester.run("$env.WHERE").expect_value_eq("mods")
}

#[test]
fn command_added_back_after_a_reload_is_visible(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'old' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu")?;
    let () = tester.run(r#""export def foo [] { 'foo' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam.nu")?;
    tester
        .run("spam bar")
        .expect_error_code_eq("nu::shell::external_command")?;
    let () = tester
        .run(r#""export def foo [] { 'foo' }; export def bar [] { 'new' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam.nu")?;
    tester.run("spam bar").expect_value_eq("new")
}

// Undoing an edit brings back the earlier version, which is still cached.
#[rstest]
#[case::prefixed("reload_reverted_prefixed", "use spam.nu", "spam bar", "spam only_v2")]
#[case::glob("reload_reverted_glob", "use spam.nu *", "bar", "only_v2")]
fn reverting_a_file_restores_its_commands(
    #[case] _topic: &str,
    #[case] import: &str,
    #[case] in_v1: &str,
    #[case] in_v2: &str,
    #[ignore] playground: Playground,
) -> Result {
    const V1: &str = "export def foo [] { 'foo' }; export def bar [] { 'bar' }";
    const V2: &str = "export def foo [] { 'foo' }; export def only_v2 [] { 'v2' }";

    playground.file("spam.nu", V1)?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run(import)?;
    let () = tester.run_with_data("save -f spam.nu", V2)?;
    let () = tester.run(import)?;
    let _: Value = tester.run(in_v2)?;
    let () = tester.run_with_data("save -f spam.nu", V1)?;
    let () = tester.run(import)?;
    tester.run(in_v1).expect_value_eq("bar")?;
    tester
        .run(in_v2)
        .expect_error_code_eq("nu::shell::external_command")?;
    // The module's name now means this version again.
    let () = tester.run("hide spam")?;
    tester
        .run(in_v1)
        .expect_error_code_eq("nu::shell::external_command")
}

// Two files that share a module name are different modules, so neither one replaces the other.
#[test]
fn other_file_with_the_same_module_name_keeps_commands(playground: Playground) -> Result {
    playground.dir("a")?;
    playground.dir("b")?;
    playground.file("a/spam.nu", "export def only_a [] { 'a' }")?;
    playground.file("b/spam.nu", "export def only_b [] { 'b' }")?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use a/spam.nu")?;
    let () = tester.run("use b/spam.nu")?;
    tester.run("spam only_a").expect_value_eq("a")?;
    tester.run("spam only_b").expect_value_eq("b")
}

#[test]
fn hide_after_a_reload_hides_every_command(playground: Playground) -> Result {
    playground.file(
        "spam.nu",
        "export def foo [] { 'foo' }; export def bar [] { 'bar' }",
    )?;

    let mut tester = test().cwd(playground.path());
    let () = tester.run("use spam.nu")?;
    let () = tester.run(r#""export def foo [] { 'foo' }" | save -f spam.nu"#)?;
    let () = tester.run("use spam.nu")?;
    let () = tester.run("hide spam")?;
    tester
        .run("spam foo")
        .expect_error_code_eq("nu::shell::external_command")?;
    tester
        .run("spam bar")
        .expect_error_code_eq("nu::shell::external_command")
}
