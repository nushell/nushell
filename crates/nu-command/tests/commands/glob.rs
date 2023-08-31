use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu};

#[test]
fn empty_glob_pattern_triggers_error() {
    Playground::setup("glob_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
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
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "glob '*' | length",
        );

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn glob_subdirs() {
    Playground::setup("glob_subdirs", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(vec![
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "glob '**/*' | length",
        );

        assert_eq!(
            actual.out, "8",
            "count must be 8 due to 6 files and 2 folders, including the cwd"
        );
    })
}

#[test]
fn glob_subdirs_ignore_dirs() {
    Playground::setup("glob_subdirs_ignore_directories", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(vec![
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "glob '**/*' -D | length",
        );

        assert_eq!(
            actual.out, "6",
            "directory count must be 6, ignoring the cwd and the children folders"
        );
    })
}

#[test]
fn glob_ignore_files() {
    Playground::setup("glob_ignore_files", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(vec![
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            ("glob '*' -F | length",
        );

        assert_eq!(
            actual.out, "1",
            "should only find one folder; ignoring cwd, files, subfolders"
        );
    })
}
