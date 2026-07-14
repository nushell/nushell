use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn ln_invalid_arg() {
    Playground::setup("uln_test_1", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "ln --definitely-invalid"
        );

        assert!(!actual.err.is_empty());
    });
}

#[test]
fn ln_extra_operand_with_no_target_directory() {
    Playground::setup("uln_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            nu_test_support::fs::Stub::EmptyFile("a.txt"),
            nu_test_support::fs::Stub::EmptyFile("b.txt"),
            nu_test_support::fs::Stub::EmptyFile("c.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ln -T a.txt b.txt c.txt"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("extra operand") && actual.err.contains("c.txt"));
    });
}

#[test]
fn ln_does_not_overwrite_existing_destination() {
    Playground::setup("uln_test_3", |dirs, sandbox| {
        sandbox.with_files(&[
            nu_test_support::fs::Stub::FileWithContent("source.txt", "source"),
            nu_test_support::fs::Stub::FileWithContent("dest.txt", "dest"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ln source.txt dest.txt"
        );

        assert!(!actual.err.is_empty());
        assert_eq!(
            std::fs::read_to_string(dirs.test().join("dest.txt")).unwrap(),
            "dest"
        );
    });
}

#[test]
fn ln_missing_destination() {
    Playground::setup("uln_test_4", |dirs, sandbox| {
        sandbox.with_files(&[nu_test_support::fs::Stub::EmptyFile("source.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "ln -s -T source.txt"
        );

        assert!(actual.err.contains("missing destination") && actual.err.contains("source.txt"));
    });
}

#[test]
fn ln_hard_link_dir_fails() {
    Playground::setup("uln_test_5", |dirs, sandbox| {
        sandbox.mkdir("dir");

        let actual = nu!(
            cwd: dirs.test(),
            "ln dir dir_link"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("hard link not allowed for directory"));
    });
}

#[test]
fn ln_multiple_sources_target_not_directory_fails() {
    Playground::setup("uln_test_6", |dirs, sandbox| {
        sandbox.with_files(&[
            nu_test_support::fs::Stub::EmptyFile("a"),
            nu_test_support::fs::Stub::EmptyFile("b"),
            nu_test_support::fs::Stub::EmptyFile("c"),
        ]);

        let actual = nu!(
            cwd: dirs.test(),
            "ln a b c"
        );
        assert!(actual.err.contains("not a directory"));
    });
}

#[test]
fn ln_force_same_file_detected_after_canonicalization() {
    Playground::setup("uln_test_7", |dirs, sandbox| {
        sandbox.with_files(&[nu_test_support::fs::Stub::FileWithContent("file", "hello")]);

        let actual = nu!(
            cwd: dirs.test(),
            "ln -f file ./file"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("same file"));
    });
}

#[test]
fn ln_relative_requires_symbolic() {
    Playground::setup("uln_test_8", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "ln -r foo bar"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("argument is required"));
    });
}

#[test]
fn ln_no_target_directory_with_directory_destination_fails() {
    Playground::setup("uln_test_9", |dirs, sandbox| {
        sandbox
            .with_files(&[nu_test_support::fs::Stub::EmptyFile("source.txt")])
            .mkdir("dst");

        let actual = nu!(
            cwd: dirs.test(),
            "ln -T source.txt dst"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("Already exists"));
    });
}

#[test]
fn ln_target_directory_flag_currently_errors() {
    Playground::setup("uln_test_10", |dirs, sandbox| {
        sandbox
            .with_files(&[
                nu_test_support::fs::Stub::EmptyFile("a.txt"),
                nu_test_support::fs::Stub::EmptyFile("b.txt"),
            ])
            .mkdir("links");

        let actual = nu!(
            cwd: dirs.test(),
            "ln -s -t links a.txt b.txt"
        );

        assert!(!actual.err.is_empty());
        assert!(actual.err.contains("unknown flag"));
    });
}
