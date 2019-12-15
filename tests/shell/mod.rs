mod pipeline {
    use test_support::fs::Stub::FileWithContent;
    use test_support::playground::Playground;
    use test_support::{nu_combined, pipeline};

    #[test]
    fn it_arg_works_with_many_inputs_to_external_command() {
        Playground::setup("it_arg_works_with_many_inputs", |dirs, sandbox| {
            sandbox.with_files(vec![
                FileWithContent("file1", "text"),
                FileWithContent("file2", " and more text"),
            ]);

            let (stdout, stderr) = nu_combined!(
                cwd: dirs.test(), pipeline(
                r#"
                    echo hello world
                    | split-row " "
                    | ^echo $it
                "#
            ));

            #[cfg(windows)]
            assert_eq!("hello world", stdout);

            #[cfg(not(windows))]
            assert_eq!("helloworld", stdout);

            assert!(!stderr.contains("No such file or directory"));
        })
    }
}
