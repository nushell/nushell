use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

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
                    export alias foo-helper = "foo"
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
                    export alias foo = "foo"
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
                    export alias bar = "bar"
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
                    export alias bar = "bar"
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
                    export alias bar = "bar"
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
    Playground::setup("module_imprt_env_1", |dirs, sandbox| {
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
