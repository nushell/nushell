use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, nu_repl_code, pipeline};
use pretty_assertions::assert_eq;

#[test]
fn module_private_import_decl() {
    Playground::setup("module_private_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    use spam.nu foo-helper

                    export def foo [] { foo-helper }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def get-foo [] { "foo" }
                    export def foo-helper [] { get-foo }
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_private_import_alias() {
    Playground::setup("module_private_import_alias", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    use spam.nu foo-helper

                    export def foo [] { foo-helper }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export alias foo-helper = echo "foo"
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_private_import_decl_not_public() {
    Playground::setup("module_private_import_decl_not_public", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    use spam.nu foo-helper
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def get-foo [] { "foo" }
                    export def foo-helper [] { get-foo }
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo-helper"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(!actual.err.is_empty());
    })
}

#[test]
fn module_public_import_decl() {
    Playground::setup("module_public_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam.nu foo
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def foo-helper [] { "foo" }
                    export def foo [] { foo-helper }
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_public_import_alias() {
    Playground::setup("module_public_import_alias", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam.nu foo
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export alias foo = echo "foo"
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_nested_imports() {
    Playground::setup("module_nested_imports", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export use spam2.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam2.nu",
                r#"
                    export use spam3.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &[r#"use main.nu foo"#, r#"foo"#];
        let inp2 = &[r#"use main.nu bar"#, r#"bar"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp1.join("; ")));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), pipeline(&inp2.join("; ")));
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
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam/spam.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam.nu",
                r#"
                    export use spam2/spam2.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam2/spam2.nu",
                r#"
                    export use ../spam3/spam3.nu [ foo bar ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam3/spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &[r#"use main.nu foo"#, r#"foo"#];
        let inp2 = &[r#"use main.nu bar"#, r#"bar"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp1.join("; ")));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), pipeline(&inp2.join("; ")));
        assert_eq!(actual.out, "bar");
    })
}

#[test]
fn module_public_import_decl_prefixed() {
    Playground::setup("module_public_import_decl", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    def foo-helper [] { "foo" }
                    export def foo [] { foo-helper }
                "#,
            )]);

        let inp = &[r#"use main.nu"#, r#"main spam foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

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
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export use spam/spam.nu [ "spam2 foo" "spam2 spam3 bar" ]
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam.nu",
                r#"
                    export use spam2/spam2.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam2/spam2.nu",
                r#"
                    export use ../spam3/spam3.nu
                    export use ../spam3/spam3.nu foo
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam/spam3/spam3.nu",
                r#"
                    export def foo [] { "foo" }
                    export alias bar = echo "bar"
                "#,
            )]);

        let inp1 = &[r#"use main.nu"#, r#"main spam2 foo"#];
        let inp2 = &[r#"use main.nu "spam2 spam3 bar""#, r#"spam2 spam3 bar"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp1.join("; ")));
        assert_eq!(actual.out, "foo");

        let actual = nu!(cwd: dirs.test(), pipeline(&inp2.join("; ")));
        assert_eq!(actual.out, "bar");
    })
}

#[test]
fn module_import_env_1() {
    Playground::setup("module_import_env_1", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export-env { source-env spam.nu }

                    export def foo [] { $env.FOO_HELPER }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export-env { let-env FOO_HELPER = "foo" }
                "#,
            )]);

        let inp = &[r#"source-env main.nu"#, r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_import_env_2() {
    Playground::setup("module_import_env_2", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    export-env { source-env spam.nu }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export-env { let-env FOO = "foo" }
                "#,
            )]);

        let inp = &[r#"source-env main.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_cyclical_imports_0() {
    Playground::setup("module_cyclical_imports_0", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                    use eggs.nu
                "#,
        )]);

        let inp = &[r#"module eggs { use spam.nu }"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("module not found"));
    })
}

#[test]
fn module_cyclical_imports_1() {
    Playground::setup("module_cyclical_imports_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                    use spam.nu
                "#,
        )]);

        let inp = &[r#"use spam.nu"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_cyclical_imports_2() {
    Playground::setup("module_cyclical_imports_2", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    use eggs.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "eggs.nu",
                r#"
                    use spam.nu
                "#,
            )]);

        let inp = &[r#"use spam.nu"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_cyclical_imports_3() {
    Playground::setup("module_cyclical_imports_3", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    use eggs.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "eggs.nu",
                r#"
                    use bacon.nu
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "bacon.nu",
                r#"
                    use spam.nu
                "#,
            )]);

        let inp = &[r#"use spam.nu"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("cyclical"));
    })
}

#[test]
fn module_import_const_file() {
    Playground::setup("module_import_const_file", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export def foo [] { "foo" }
            "#,
        )]);

        let inp = &[r#"const file = 'spam.nu'"#, r#"use $file foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_import_const_module_name() {
    Playground::setup("module_import_const_file", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export def foo [] { "foo" }
            "#,
        )]);

        let inp = &[
            r#"module spam { export def foo [] { "foo" } }"#,
            r#"const mod = 'spam'"#,
            r#"use $mod foo"#,
            r#"foo"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn module_valid_def_name() {
    let inp = &[r#"module spam { def spam [] { "spam" } }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_def_name() {
    let inp = &[r#"module spam { export def spam [] { "spam" } }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_valid_alias_name_1() {
    let inp = &[r#"module spam { alias spam = echo "spam" }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "");
}

#[test]
fn module_valid_alias_name_2() {
    let inp = &[r#"module spam { alias main = echo "spam" }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_alias_name() {
    let inp = &[r#"module spam { export alias spam = echo "spam" }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_main_alias_not_allowed() {
    let inp = &[r#"module spam { export alias main = echo 'spam' }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("export_main_alias_not_allowed"));
}

#[test]
fn module_valid_known_external_name() {
    let inp = &[r#"module spam { extern spam [] }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "");
}

#[test]
fn module_invalid_known_external_name() {
    let inp = &[r#"module spam { export extern spam [] }"#];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn main_inside_module_is_main() {
    let inp = &[
        r#"module spam {
            export def main [] { 'foo' };
            export def foo [] { main }
        }"#,
        "use spam foo",
        "foo",
    ];

    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "foo");
}

#[test]
fn module_as_file() {
    let inp = &[r#"module samples/spam.nu"#, "use spam foo", "foo"];

    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "foo");
}

#[test]
fn export_module_as_file() {
    let inp = &[r#"export module samples/spam.nu"#, "use spam foo", "foo"];

    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "foo");
}

#[test]
fn deep_import_patterns() {
    let module_decl = r#"
        module spam {
            export module eggs {
                export module beans {
                    export def foo [] { 'foo' };
                    export def bar [] { 'bar' }
                };
            };
        }
    "#;

    let inp = &[module_decl, "use spam", "spam eggs beans foo"];
    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs", "eggs beans foo"];
    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs beans", "beans foo"];
    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foo");

    let inp = &[module_decl, "use spam eggs beans foo", "foo"];
    let actual = nu!(cwd: ".", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foo");
}

#[test]
fn module_dir() {
    let import = "use samples/spam";

    let inp = &[import, "spam"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "spam");

    let inp = &[import, "spam foo"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foo");

    let inp = &[import, "spam bar"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "bar");

    let inp = &[import, "spam foo baz"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "foobaz");

    let inp = &[import, "spam bar baz"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "barbaz");

    let inp = &[import, "spam baz"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "spambaz");
}

#[test]
fn module_dir_deep() {
    let import = "use samples/spam";

    let inp = &[import, "spam bacon"];
    let actual_repl = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual_repl.out, "bacon");

    let inp = &[import, "spam bacon foo"];
    let actual_repl = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual_repl.out, "bacon foo");

    let inp = &[import, "spam bacon beans"];
    let actual_repl = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual_repl.out, "beans");

    let inp = &[import, "spam bacon beans foo"];
    let actual_repl = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
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
fn not_allowed_submodule_file() {
    let inp = &["use samples/not_allowed"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("invalid_module_file_name"));
}

#[test]
fn module_dir_missing_mod_nu() {
    let inp = &["use samples/missing_mod_nu"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("module_missing_mod_nu_file"));
}

#[test]
fn allowed_local_module() {
    let inp = &["module spam { module spam {} }"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.is_empty());
}

#[test]
fn not_allowed_submodule() {
    let inp = &["module spam { export module spam {} }"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("named_as_module"));
}

#[test]
fn module_self_name() {
    let inp = &[
        "module spam { export module mod { export def main [] { 'spam' } } }",
        "use spam",
        "spam",
    ];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert_eq!(actual.out, "spam");
}

#[test]
fn module_self_name_main_not_allowed() {
    let inp = &[
        r#"module spam {
            export def main [] { 'main spam' };

            export module mod {
                export def main [] { 'mod spam' }
            }
        }"#,
        "use spam",
        "spam",
    ];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("module_double_main"));

    let inp = &[
        r#"module spam {
            export module mod {
                export def main [] { 'mod spam' }
            };

            export def main [] { 'main spam' }
        }"#,
        "use spam",
        "spam",
    ];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("module_double_main"));
}

#[test]
fn module_main_not_found() {
    let inp = &["module spam {}", "use spam main"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("export_not_found"));

    let inp = &["module spam {}", "use spam [ main ]"];
    let actual = nu!(cwd: "tests/modules", pipeline(&inp.join("; ")));
    assert!(actual.err.contains("export_not_found"));
}
