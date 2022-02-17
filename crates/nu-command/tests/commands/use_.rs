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
