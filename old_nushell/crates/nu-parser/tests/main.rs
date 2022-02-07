use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn defs_contain_comment_in_help() {
    Playground::setup("comment_test", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "my_def.nu",
            r#"
                # I comment and test. I am a good boy.
                def comment_philosphy [] {
                    echo It’s not a bug – it’s an undocumented feature. (Anonymous)
                }
                "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            source my_def.nu
            help comment_philosphy
            "#);

        assert!(actual.out.contains("I comment and test. I am a good boy."));
    });
}

#[test]
fn defs_contain_multiple_comments_in_help() {
    Playground::setup("comment_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "my_def.nu",
            r#"
                # I comment and test. I am a good boy.
                def comment_philosphy [] {
                    echo It’s not a bug – it’s an undocumented feature. (Anonymous)
                }

                # I comment and test all my functions. I am a very good boy.
                def comment_philosphy_2 [] {
                    echo It’s not a bug – it’s an undocumented feature. (Anonymous)
                }


                # I comment and test all my functions. I am the best boy.
                def comment_philosphy_3 [] {
                    echo It’s not a bug – it’s an undocumented feature. (Anonymous)
                }
                "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            source my_def.nu
            help comment_philosphy
            help comment_philosphy_2
            help comment_philosphy_3
            "#);

        assert!(actual.out.contains("I comment and test. I am a good boy."));
        assert!(actual
            .out
            .contains("I comment and test all my functions. I am a very good boy."));
        assert!(actual
            .out
            .contains("I comment and test all my functions. I am the best boy."));
    });
}
