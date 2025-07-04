use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use rstest::rstest;
use std::path::{Path, PathBuf};

#[test]
fn empty_glob_pattern_triggers_error() {
    Playground::setup("glob_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
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
        sandbox.with_files(&[
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
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
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
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
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
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);
        sandbox.mkdir("children");
        sandbox.within("children").with_files(&[
            EmptyFile("timothy.txt"),
            EmptyFile("tiffany.txt"),
            EmptyFile("trish.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "glob '*' -F | length",
        );

        assert_eq!(
            actual.out, "1",
            "should only find one folder; ignoring cwd, files, subfolders"
        );
    })
}

// clone of fs::create_file_at removing the parent panic, whose purpose I do not grok.
pub fn create_file_at(full_path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let full_path = full_path.as_ref();
    std::fs::write(full_path, b"fake data")
}

// playground has root directory and subdirectories foo and foo/bar to play with
// specify all test files relative to root directory.
// OK to use fwd slash in paths, they're hacked to OS dir separator when needed (windows)
#[rstest]
#[case(".", r#"'*z'"#, &["ablez", "baker", "charliez"], &["ablez", "charliez"], "simple glob")]
#[case(".", r#"'qqq'"#, &["ablez", "baker", "charliez"], &[], "glob matches none")]
#[case("foo/bar", r"'*[\]}]*'", &[r#"foo/bar/ab}le"#, "foo/bar/baker", r#"foo/bar/cha]rlie"#], &[r#"foo/bar/ab}le"#, r#"foo/bar/cha]rlie"#], "glob has quoted metachars")]
#[case("foo/bar", r#"'../*'"#, &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/able", "foo/bar", "foo/charlie"], "glob matches files in parent")]
#[case("foo", r#"'./{a,b}*'"#, &["foo/able", "foo/bar/baker", "foo/charlie"], &["foo/able", "foo/bar"], "glob with leading ./ matches peer files")]
fn glob_files_in_parent(
    #[case] wd: &str,
    #[case] glob: &str,
    #[case] ini: &[&str],
    #[case] exp: &[&str],
    #[case] tag: &str,
) {
    Playground::setup("glob_test", |dirs, sandbox| {
        sandbox.within("foo").within("bar");
        let working_directory = &dirs.test().join(wd);

        for f in ini {
            create_file_at(dirs.test().join(f)).expect("couldn't create file");
        }

        let actual = nu!(
            cwd: working_directory,
            r#"glob {} | sort | str join " ""#,
            glob
        );

        let mut expected: Vec<String> = vec![];
        for e in exp {
            expected.push(
                dirs.test()
                    .join(PathBuf::from(e)) // sadly, does *not" convert "foo/bar" to "foo\\bar" on Windows.
                    .to_string_lossy()
                    .to_string(),
            );
        }

        let expected = expected
            .join(" ")
            .replace('/', std::path::MAIN_SEPARATOR_STR);
        assert_eq!(actual.out, expected, "\n  test: {tag}");
    });
}

#[test]
fn glob_follow_symlinks() {
    Playground::setup("glob_follow_symlinks", |dirs, sandbox| {
        // Create a directory with some files
        sandbox.mkdir("target_dir");
        sandbox
            .within("target_dir")
            .with_files(&[EmptyFile("target_file.txt")]);

        let target_dir = dirs.test().join("target_dir");
        let symlink_path = dirs.test().join("symlink_dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(target_dir, &symlink_path).expect("Failed to create symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(target_dir, &symlink_path)
            .expect("Failed to create symlink");

        // on some systems/filesystems, symlinks are followed by default
        // on others (like Linux /sys), they aren't
        // Test that with the --follow-symlinks flag, files are found for sure
        let with_flag = nu!(
            cwd: dirs.test(),
            "glob 'symlink_dir/*.txt' --follow-symlinks | length",
        );

        assert_eq!(
            with_flag.out, "1",
            "Should find file with --follow-symlinks flag"
        );
    })
}
