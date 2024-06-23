use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code};
use pretty_assertions::assert_eq;

#[test]
fn module_private_import_decl() {
    Playground::setup("module_private_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    use spam.nu foo-helper

                    export def foo [] { foo-helper }
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def get-foo [] { "foo" }
                    export def foo-helper [] { get-foo }
                "#,
            )]);

        let inp = &["use main.nu foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_private_import_alias() {
    Playground::setup("module_private_import_alias", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    use spam.nu foo-helper

                    export def foo [] { foo-helper }
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export alias foo-helper = echo "foo"
                "#,
            )]);

        let inp = &["use main.nu foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_private_import_decl_not_public() {
    Playground::setup("module_private_import_decl_not_public", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    use spam.nu foo-helper
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def get-foo [] { "foo" }
                    export def foo-helper [] { get-foo }
                "#,
            )]);

        let inp = &["use main.nu foo", "foo-helper"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(!actual.err.is_empty());
    })
}

#[test]
fn module_public_import_decl() {
    Playground::setup("module_public_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export use spam.nu foo
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def foo-helper [] { "foo" }
                    export def foo [] { foo-helper }
                "#,
            )]);

        let inp = &["use main.nu foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_public_import_alias() {
    Playground::setup("module_public_import_alias", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export use spam.nu foo
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export alias foo = echo "foo"
                "#,
            )]);

        let inp = &["use main.nu foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_nested_imports() {
    Playground::setup("module_nested_imports", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export use spam.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                "
                    export use spam2.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam2.nu",
                "
                    export use spam3.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &["use main.nu foo", "foo"];
        let inp2 = &["use main.nu bar", "bar"];

        let actual = nu!(cwd: dirs.test(), &inp1.join("; "));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), &inp2.join("; "));
        assert_eq!(actual.out, "bar");
    })
}

#[test]
fn module_nested_imports_in_dirs() {
    Playground::setup("module_nested_imports_in_dirs", |dirs, sandbox| {
        sandbox
            .mkdir("spam")
            .mkdir("spam/spam2")
            .mkdir("spam/spam3")
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export use spam/spam.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam.nu",
                "
                    export use spam2/spam2.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam2/spam2.nu",
                "
                    export use ../spam3/spam3.nu [ foo bar ]
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam3/spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &["use main.nu foo", "foo"];
        let inp2 = &["use main.nu bar", "bar"];

        let actual = nu!(cwd: dirs.test(), &inp1.join("; "));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), &inp2.join("; "));
        assert_eq!(actual.out, "bar");
    })
}

#[test]
fn module_public_import_decl_prefixed() {
    Playground::setup("module_public_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export use spam.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def foo-helper [] { "foo" }
                    export def foo [] { foo-helper }
                "#,
            )]);

        let inp = &["use main.nu", "main spam foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_nested_imports_in_dirs_prefixed() {
    Playground::setup("module_nested_imports_in_dirs", |dirs, sandbox| {
        sandbox
            .mkdir("spam")
            .mkdir("spam/spam2")
            .mkdir("spam/spam3")
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam/spam.nu [ "spam2 foo" "spam2 spam3 bar" ]
                "#,
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam.nu",
                "
                    export use spam2/spam2.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam2/spam2.nu",
                "
                    export use ../spam3/spam3.nu
                    export use ../spam3/spam3.nu foo
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam/spam3/spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &["use main.nu", "main spam2 foo"];
        let inp2 = &[r#"use main.nu "spam2 spam3 bar""#, "spam2 spam3 bar"];

        let actual = nu!(cwd: dirs.test(), &inp1.join("; "));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), &inp2.join("; "));
        assert_eq!(actual.out, "bar");
    })
}

#[test]
fn module_import_env_1() {
    Playground::setup("module_import_env_1", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export-env { source-env spam.nu }

                    export def foo [] { $env.FOO_HELPER }
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export-env { $env.FOO_HELPER = "foo" }
                "#,
            )]);

        let inp = &["source-env main.nu", "use main.nu foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_import_env_2() {
    Playground::setup("module_import_env_2", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "main.nu",
                "
                    export-env { source-env spam.nu }
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export-env { $env.FOO = "foo" }
                "#,
            )]);

        let inp = &["source-env main.nu", "$env.FOO"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_cyclical_imports_0() {
    Playground::setup("module_cyclical_imports_0", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                    use eggs.nu
                ",
        )]);

        let inp = &["module eggs { use spam.nu }"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(actual.err.contains("Module not found"));
    })
}

#[test]
fn module_cyclical_imports_1() {
    Playground::setup("module_cyclical_imports_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                    use spam.nu
                ",
        )]);

        let inp = &["use spam.nu"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_cyclical_imports_2() {
    Playground::setup("module_cyclical_imports_2", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                "
                    use eggs.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "eggs.nu",
                "
                    use spam.nu
                ",
            )]);

        let inp = &["use spam.nu"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_cyclical_imports_3() {
    Playground::setup("module_cyclical_imports_3", |dirs, sandbox| {
        sandbox
            .with_files(&[FileWithContentToBeTrimmed(
                "spam.nu",
                "
                    use eggs.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "eggs.nu",
                "
                    use bacon.nu
                ",
            )])
            .with_files(&[FileWithContentToBeTrimmed(
                "bacon.nu",
                "
                    use spam.nu
                ",
            )]);

        let inp = &["use spam.nu"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_import_const_file() {
    Playground::setup("module_import_const_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export def foo [] { "foo" }
            "#,
        )]);

        let inp = &["const file = 'spam.nu'", "use $file foo", "foo"];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_import_const_module_name() {
    Playground::setup("module_import_const_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export def foo [] { "foo" }
            "#,
        )]);

        let inp = &[
            r#"module spam { export def foo [] { "foo" } }"#,
            "const mod = 'spam'",
            "use $mod foo",
            "foo",
        ];

        let actual = nu!(cwd: dirs.test(), &inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_valid_def_name() {
    let inp = &[r#"module spam { def spam [] { "spam" } }"#];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_def_name() {
    let inp = &[r#"module spam { export def spam [] { "spam" } }"#];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_valid_alias_name_1() {
    let inp = &[r#"module spam { alias spam = echo "spam" }"#];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "");
}

#[test]
fn module_valid_alias_name_2() {
    let inp = &[r#"module spam { alias main = echo "spam" }"#];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_alias_name() {
    let inp = &[r#"module spam { export alias spam = echo "spam" }"#];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_main_alias_not_allowed() {
    let inp = &["module spam { export alias main = echo 'spam' }"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("export_main_alias_not_allowed"));
}

#[test]
fn module_valid_known_external_name() {
    let inp = &["module spam { extern spam [] }"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_known_external_name() {
    let inp = &["module spam { export extern spam [] }"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn main_inside_module_is_main() {
    let inp = &[
        "module spam {
            export def main [] { 'foo' };
            export def foo [] { main }
        }",
        "use spam foo",
        "foo",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "foo");
}

#[test]
fn module_as_file() {
    let inp = &["module samples/spam.nu", "use spam foo", "foo"];

    let actual = nu!(cwd: "tests/modules", &inp.join("; "));

    assert_eq!(actual.out, "foo");
}

#[test]
fn export_module_as_file() {
    let inp = &["export module samples/spam.nu", "use spam foo", "foo"];

    let actual = nu!(cwd: "tests/modules", &inp.join("; "));

    assert_eq!(actual.out, "foo");
}

#[test]
fn deep_import_patterns() {
    let module_decl = "
        module spam {
            export module eggs {
                export module beans {
                    export def foo [] { 'foo' };
                    export def bar [] { 'bar' }
                };
            };
        }
    ";

    let inp = &[module_decl, "use spam", "spam eggs beans foo"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs", "eggs beans foo"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs beans", "beans foo"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs beans foo", "foo"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "foo");
}

#[test]
fn module_dir() {
    let import = "use samples/spam";

    let inp = &[import, "spam"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spam");

    let inp = &[import, "spam foo"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foo");

    let inp = &[import, "spam bar"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "bar");

    let inp = &[import, "spam foo baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "foobaz");

    let inp = &[import, "spam bar baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "barbaz");

    let inp = &[import, "spam baz"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual.out, "spambaz");
}

#[test]
fn module_dir_deep() {
    let import = "use samples/spam";

    let inp = &[import, "spam bacon"];
    let actual_repl = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual_repl.out, "bacon");

    let inp = &[import, "spam bacon foo"];
    let actual_repl = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual_repl.out, "bacon foo");

    let inp = &[import, "spam bacon beans"];
    let actual_repl = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual_repl.out, "beans");

    let inp = &[import, "spam bacon beans foo"];
    let actual_repl = nu!(cwd: "tests/modules", &inp.join("; "));
    assert_eq!(actual_repl.out, "beans foo");
}

#[test]
fn module_dir_import_twice_no_panic() {
    let import = "use samples/spam";
    let inp = &[import, import, "spam"];
    let actual_repl = nu!(cwd: "tests/modules", nu_repl_code(inp));
    assert_eq!(actual_repl.out, "spam");
}

#[test]
fn module_dir_missing_mod_nu() {
    let inp = &["use samples/missing_mod_nu"];
    let actual = nu!(cwd: "tests/modules", &inp.join("; "));
    assert!(actual.err.contains("module_missing_mod_nu_file"));
}

#[test]
fn allowed_local_module() {
    let inp = &["module spam { module spam {} }"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.is_empty());
}

#[test]
fn not_allowed_submodule() {
    let inp = &["module spam { export module spam {} }"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_self_name() {
    let inp = &[
        "module spam { export module mod { export def main [] { 'spam' } } }",
        "use spam",
        "spam",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "spam");
}

#[test]
fn module_self_name_main_not_allowed() {
    let inp = &[
        "module spam {
            export def main [] { 'main spam' };

            export module mod {
                export def main [] { 'mod spam' }
            }
        }",
        "use spam",
        "spam",
    ];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("module_double_main"));

    let inp = &[
        "module spam {
            export module mod {
                export def main [] { 'mod spam' }
            };

            export def main [] { 'main spam' }
        }",
        "use spam",
        "spam",
    ];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("module_double_main"));
}

#[test]
fn module_main_not_found() {
    let inp = &["module spam {}", "use spam main"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("export_not_found"));

    let inp = &["module spam {}", "use spam [ main ]"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("export_not_found"));
}

#[test]
fn nested_list_export_works() {
    let module = r#"
        module spam {
            export module eggs {
                export def bacon [] { 'bacon' }
            }

            export def sausage [] { 'sausage' }
        }
    "#;

    let inp = &[module, "use spam [sausage eggs]", "eggs bacon"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "bacon");
}

#[test]
fn reload_submodules() {
    Playground::setup("reload_submodule_changed_file", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export module animals.nu"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);

        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        // should also verify something unchanged if `use voice`.
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'meow'}" | save -f animals.nu"#,
            "use voice",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        // should also works if we use members directly.
        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export module animals.nu"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu animals cat",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu animals cat",
            "(cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");
    });
}

#[test]
fn use_submodules() {
    Playground::setup("use_submodules", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);

        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        // should also verify something unchanged if `use voice`.
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'meow'}" | save -f animals.nu"#,
            "use voice",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        // also verify something is changed when using members.
        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu cat"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu",
            "(voice cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu *"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu",
            "(voice cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu [cat]"#),
            FileWithContent("animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f animals.nu"#,
            "use voice.nu",
            "(voice cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");
    });
}

#[test]
fn use_nested_submodules() {
    Playground::setup("use_submodules", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu"#),
            FileWithContent("animals.nu", r#"export use nested_animals.nu"#),
            FileWithContent("nested_animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f nested_animals.nu"#,
            "use voice.nu",
            "(voice animals nested_animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        sandbox.with_files(&[
            FileWithContent("voice.nu", r#"export use animals.nu"#),
            FileWithContent("animals.nu", r#"export use nested_animals.nu cat"#),
            FileWithContent("nested_animals.nu", "export def cat [] { 'meow'}"),
        ]);
        let inp = [
            "use voice.nu",
            r#""export def cat [] {'woem'}" | save -f nested_animals.nu"#,
            "use voice.nu",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");

        sandbox.with_files(&[
            FileWithContent("animals.nu", r#"export use nested_animals.nu cat"#),
            FileWithContent("nested_animals.nu", "export def cat [] { 'meow' }"),
        ]);
        let inp = [
            "module voice { export module animals.nu }",
            "use voice",
            r#""export def cat [] {'woem'}" | save -f nested_animals.nu"#,
            "use voice.nu",
            "(voice animals cat) == 'woem'",
        ];
        let actual = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual.out, "true");
    })
}
