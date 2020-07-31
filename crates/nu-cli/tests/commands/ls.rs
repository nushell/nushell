use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::{Dirs, Playground};
use nu_test_support::{nu, pipeline};

#[test]
fn lists_regular_files() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_regular_files_using_asterisk_wildcard() {
    Playground::setup("ls_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.txt
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_regular_files_using_question_mark_wildcard() {
    Playground::setup("ls_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.10.txt"),
            EmptyFile("jonathan.10.txt"),
            EmptyFile("andres.10.txt"),
            EmptyFile("chicken_not_to_be_picked_up.100.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls *.??.txt
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn lists_all_files_in_directories_from_stream() {
    Playground::setup("ls_test_4", |dirs, sandbox| {
        sandbox
            .with_files(vec![EmptyFile("root1.txt"), EmptyFile("root2.txt")])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
            ])
            .within("dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo dir_a dir_b
                | ls $it
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn does_not_fail_if_glob_matches_empty_directory() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.within("dir_a");

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls dir_a
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "0");
    })
}

#[test]
fn fails_when_glob_doesnt_match() {
    Playground::setup("ls_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("root1.txt"), EmptyFile("root2.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "ls root3*"
        );

        assert!(actual.err.contains("no matches found"));
    })
}

#[test]
fn list_files_from_two_parents_up_using_multiple_dots() {
    Playground::setup("ls_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yahuda.yaml"),
            EmptyFile("jonathan.json"),
            EmptyFile("andres.xml"),
            EmptyFile("kevin.txt"),
        ]);

        sandbox.within("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test().join("foo/bar"),
            r#"
                ls ... | count | echo $it
            "#
        );

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn list_all_columns() {
    Playground::setup(
        "ls_test_all_columns",
        |dirs: Dirs, sandbox: &mut Playground| {
            sandbox.with_files(vec![
                EmptyFile("Leonardo.yaml"),
                EmptyFile("Raphael.json"),
                EmptyFile("Donatello.xml"),
                EmptyFile("Michelangelo.txt"),
            ]);
            // Normal Operation
            let actual = nu!(
                cwd: dirs.test(),
                "ls | get | to md"
            );
            let expected = ["name", "type", "size", "modified"].join("");
            assert_eq!(actual.out, expected, "column names are incorrect for ls");
            // Symbolic Links
            let actual = nu!(
                cwd: dirs.test(),
                "ls -w | get | to md"
            );
            let expected = ["name", "type", "target", "size", "modified"].join("");
            assert_eq!(actual.out, expected, "column names are incorrect for ls -w");
            // Long
            let actual = nu!(
                cwd: dirs.test(),
                "ls -l | get | to md"
            );
            let expected = {
                #[cfg(unix)]
                {
                    [
                        "name", "type", "target", "readonly", "mode", "uid", "group", "size",
                        "created", "accessed", "modified",
                    ]
                    .join("")
                }

                #[cfg(windows)]
                {
                    [
                        "name", "type", "target", "readonly", "size", "created", "accessed",
                        "modified",
                    ]
                    .join("")
                }
            };
            assert_eq!(
                actual.out, expected,
                "column names are incorrect for ls long"
            );
        },
    );
}
