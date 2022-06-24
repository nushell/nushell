use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn parse_script_success() {
    Playground::setup("nu_check_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check script.nu
            "#
        ));

        assert_eq!(actual.out, "Parse Success!".to_string());
    })
}

#[test]
fn parse_script_with_wrong_type() {
    Playground::setup("nu_check_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name] {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module script.nu
            "#
        ));

        assert!(actual
            .err
            .contains("If the content is a script, please remove flag"));
    })
}
#[test]
fn parse_script_failure() {
    Playground::setup("nu_check_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "script.nu",
            r#"
                greet "world"

                def greet [name {
                  echo "hello" $name
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check script.nu
            "#
        ));

        assert!(actual.err.contains("Unexpected end of code"));
    })
}

#[test]
fn parse_module_success() {
    Playground::setup("nu_check_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string] {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module foo.nu
            "#
        ));

        assert_eq!(actual.out, "Parse Success!".to_string());
    })
}

#[test]
fn parse_module_with_wrong_type() {
    Playground::setup("nu_check_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check foo.nu
            "#
        ));

        assert!(actual
            .err
            .contains("If the content is a module, please use --as-module flag"));
    })
}
#[test]
fn parse_module_failure() {
    Playground::setup("nu_check_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export def hello [name: string {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module foo.nu
            "#
        ));

        assert!(actual.err.contains("Unexpected end of code"));
    })
}

#[test]
fn file_not_exist() {
    Playground::setup("nu_check_test_7", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module foo.nu
            "#
        ));

        assert!(actual.err.contains("file not found"));
    })
}

#[test]
fn parse_unsupported_file() {
    Playground::setup("nu_check_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "foo.txt",
            r#"
                # foo.nu

                export def hello [name: string {
                    $"hello ($name)!"
                }

                export def hi [where: string] {
                    $"hi ($where)!"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module foo.txt
            "#
        ));

        assert!(actual.err.contains("File extension must be .nu"));
    })
}
#[test]
fn parse_dir_failure() {
    Playground::setup("nu_check_test_9", |dirs, _sandbox| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module ~
            "#
        ));

        assert!(actual.err.contains("Path is not a file"));
    })
}

#[test]
fn parse_module_success_2() {
    Playground::setup("nu_check_test_10", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "foo.nu",
            r#"
                # foo.nu

                export env MYNAME { "Arthur, King of the Britons" }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                nu check --as-module foo.nu
            "#
        ));

        assert_eq!(actual.out, "Parse Success!".to_string());
    })
}
