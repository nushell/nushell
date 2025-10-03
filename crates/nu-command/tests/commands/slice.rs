use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn selects_a_row() {
    Playground::setup("slice_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("notes.txt"), EmptyFile("tests.txt")]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | sort-by name
            | slice 0..0
            | get name.0
        ");

        assert_eq!(actual.out, "notes.txt");
    });
}

#[test]
fn selects_some_rows() {
    Playground::setup("slice_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("notes.txt"),
            EmptyFile("tests.txt"),
            EmptyFile("persons.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | get name
            | slice 1..2
            | length
        ");

        assert_eq!(actual.out, "2");
    });
}

#[test]
fn negative_indices() {
    Playground::setup("slice_test_negative_indices", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("notes.txt"),
            EmptyFile("tests.txt"),
            EmptyFile("persons.txt"),
        ]);

        let actual = nu!(cwd: dirs.test(), "
            ls
            | get name
            | slice (-1..)
            | length
        ");

        assert_eq!(actual.out, "1");
    });
}

#[test]
fn zero_to_zero_exclusive() {
    let actual = nu!(r#"[0 1 2 3] | slice 0..<0 | to nuon"#);

    assert_eq!(actual.out, "[]");
}

#[test]
fn to_negative_one_inclusive() {
    let actual = nu!(r#"[0 1 2 3] | slice 2..-1 | to nuon"#);

    assert_eq!(actual.out, "[2, 3]");
}
