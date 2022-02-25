use nu_test_support::fs::{AbsolutePath, DisplayPath, Stub::FileWithContent};
use nu_test_support::nu;
use nu_test_support::pipeline;
use nu_test_support::playground::Playground;

#[test]
fn use_module_file_within_block() {
    Playground::setup("use_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("spam.nu"));

        nu.with_files(vec![FileWithContent(
            &file.display_path(),
            r#"
                export def foo [] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
                r#"
                    def bar [] {
                        use spam.nu foo;
                        foo
                    };
                    bar
                "#
            )
        );

        assert_eq!(actual.out, "hello world");
    })
}

#[test]
fn use_keeps_doc_comments() {
    Playground::setup("use_doc_comments", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("spam.nu"));

        nu.with_files(vec![FileWithContent(
            &file.display_path(),
            r#"
                # this is my foo command
                export def foo [
                    x:string # this is an x parameter
                ] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
                r#"
                    use spam.nu foo;
                    help foo
                "#
            )
        );

        assert!(actual.out.contains("this is my foo command"));
        assert!(actual.out.contains("this is an x parameter"));
    })
}
