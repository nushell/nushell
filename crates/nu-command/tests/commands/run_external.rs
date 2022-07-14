use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn better_empty_redirection() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            ls | each { |it| nu --testbin cococo $it.name }
        "#
    ));

    eprintln!("out: {}", actual.out);

    assert!(!actual.out.contains('2'));
}

#[cfg(not(windows))]
#[test]
fn explicit_glob() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls | glob '*.txt' | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[cfg(not(windows))]
#[test]
fn bare_word_expand_path_glob() {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls *.txt
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(not(windows))]
#[test]
fn backtick_expand_path_glob() {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls `*.txt`
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(not(windows))]
#[test]
fn single_quote_does_not_expand_path_glob() {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls '*.txt'
            "#
        ));

        assert!(actual.err.contains("No such file or directory"));
    })
}

#[cfg(not(windows))]
#[test]
fn double_quote_does_not_expand_path_glob() {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^ls "*.txt"
            "#
        ));

        assert!(actual.err.contains("No such file or directory"));
    })
}

#[cfg(windows)]
#[test]
fn explicit_glob_windows() {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir | glob '*.txt' | length
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}

#[cfg(windows)]
#[test]
fn bare_word_expand_path_glob_windows() {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir *.txt
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(windows)]
#[test]
fn backtick_expand_path_glob_windows() {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir `*.txt`
            "#
        ));

        assert!(actual.out.contains("D&D_volume_1.txt"));
        assert!(actual.out.contains("D&D_volume_2.txt"));
    })
}

#[cfg(windows)]
#[test]
fn single_quote_does_not_expand_path_glob_windows() {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir '*.txt'
            "#
        ));

        assert!(actual.err.contains("File Not Found"));
    })
}

#[cfg(windows)]
#[test]
fn double_quote_does_not_expand_path_glob_windows() {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ^dir "*.txt"
            "#
        ));

        assert!(actual.err.contains("File Not Found"));
    })
}
