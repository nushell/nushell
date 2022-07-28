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

// TODO -- doesn't work because modules are never evaluated
#[ignore]
#[test]
fn module_private_import_env() {
    Playground::setup("module_private_import_env", |dirs, sandbox| {
        sandbox
            .with_files(vec![FileWithContentToBeTrimmed(
                "main.nu",
                r#"
                    use spam.nu FOO_HELPER

                    export def foo [] { $env.FOO_HELPER }
                "#,
            )])
            .with_files(vec![FileWithContentToBeTrimmed(
                "spam.nu",
                r#"
                    export env FOO_HELPER { "foo" }
                "#,
            )]);

        let inp = &[r#"use main.nu foo"#, r#"foo"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}
