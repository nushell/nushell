use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn empty_glob_pattern_triggers_error() {
    Playground::setup("glob_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "glob ''",
        );

        assert!(actual.err.contains("must not be empty"));
    })
}

#[test]
fn nonempty_glob_lists_matching_paths() {
    Playground::setup("glob_sanity_star", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            pipeline("glob '*' | length"),
        );

        assert_eq!(actual.out, "3");
    })
}
