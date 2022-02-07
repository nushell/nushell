use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_all_rows_by_every_zero() {
    Playground::setup("every_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 0
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(
            actual.out,
            r#"["amigos.txt","arepas.clu","los.txt","tres.txt"]"#
        );
    })
}

#[test]
fn gets_no_rows_by_every_skip_zero() {
    Playground::setup("every_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 0 --skip
<<<<<<< HEAD
                | to json
            "#
        ));

        assert_eq!(actual.out, "");
=======
                | to json --raw
            "#
        ));

        assert_eq!(actual.out, "[]");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    })
}

#[test]
fn gets_all_rows_by_every_one() {
    Playground::setup("every_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 1
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(
            actual.out,
            r#"["amigos.txt","arepas.clu","los.txt","tres.txt"]"#
        );
    })
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn gets_no_rows_by_every_skip_one() {
    Playground::setup("every_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 1 --skip
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(actual.out, "");
    })
}

#[test]
fn gets_first_row_by_every_too_much() {
    Playground::setup("every_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 999
            "#
        ));

        let expected = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo [ amigos.txt ]
            "#
        ));

        assert_eq!(actual.out, expected.out);
    })
}

#[test]
fn gets_all_rows_except_first_by_every_skip_too_much() {
    Playground::setup("every_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 999 --skip
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(actual.out, r#"["arepas.clu","los.txt","tres.txt"]"#);
    })
}

#[test]
fn gets_every_third_row() {
    Playground::setup("every_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("quatro.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 3
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(actual.out, r#"["amigos.txt","quatro.txt"]"#);
    })
}

#[test]
fn skips_every_third_row() {
    Playground::setup("every_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile("los.txt"),
            EmptyFile("quatro.txt"),
            EmptyFile("tres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | every 3 --skip
<<<<<<< HEAD
                | to json
=======
                | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(actual.out, r#"["arepas.clu","los.txt","tres.txt"]"#);
    })
}
