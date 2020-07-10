use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn autoenv() {
    Playground::setup("autoenv_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar");
        sandbox.with_files(vec![
            FileWithContent(
                ".nu-env",
                r#"[env]
                    testkey = "testvalue"
                    [scriptvars]
                    myscript = "echo 'myval'"

                    [scripts]
                    entryscripts = ["touch hello.txt"]
                    exitscripts = ["touch bye.txt"]"#,
            ),
            FileWithContent(
                "foo/.nu-env",
                r#"[env]
                    overwrite_me = "set_in_foo"
                    fookey = "fooval""#,
            ),
            FileWithContent(
                "foo/bar/.nu-env",
                r#"[env]
                    overwrite_me = "set_in_bar""#,
            ),
        ]);

        //Make sure basic keys are set
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust
               echo $nu.env.testkey"#
        );
        assert!(actual.out.ends_with("testvalue"));

        // Make sure script keys are set
        let actual = nu!(
            cwd: dirs.test(),
            r#"echo $nu.env.myscript"#
        );
        assert!(actual.out.ends_with("myval"));

        // Make sure entry scripts are run
        let actual = nu!(
            cwd: dirs.test(),
            r#"ls | where name == "hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        //Backing out of the directory should unset the keys
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd ..
               echo $nu.env.testkey"#
        );
        assert!(!actual.out.ends_with("testvalue"));

        // Make sure exit scripts are run
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd ..
               ls | where name == "bye.txt" | get name"#
        );
        assert!(actual.out.contains("bye.txt"));

        //Subdirectories should overwrite the values of parent directories.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust foo
                   cd foo/bar
                   autoenv trust
                   echo $nu.env.overwrite_me"#
        );
        assert!(actual.out.ends_with("set_in_bar"));

        //Variables set in parent directories should be set even if you directly cd to a subdir
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust foo
                   cd foo/bar
                   autoenv trust
                   echo $nu.env.fookey"#
        );
        assert!(actual.out.ends_with("fooval"));

        //Make sure that overwritten values are restored.
        //By deleting foo/.nu-env, we make sure that the value is actually restored and not just set again by autoenv when we re-visit foo.
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd foo
                   cd bar
                   rm ../.nu-env
                   cd ..
                   echo $nu.env.overwrite_me"#
        );
        assert!(actual.out.ends_with("set_in_foo"))
    })
}
