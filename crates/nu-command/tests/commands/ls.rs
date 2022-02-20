use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
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
                | length
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
                | length
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
                | length
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
                | each { |it| ls $it }
                | flatten | length
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
                | length
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
                ls ... | length
            "#
        );

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn lists_hidden_file_when_explicitly_specified() {
    Playground::setup("ls_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
            EmptyFile(".testdotfile"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls .testdotfile
                | length
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn lists_all_hidden_files_when_glob_contains_dot() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within("dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls **/.*
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
// TODO Remove this cfg value when we have an OS-agnostic way
// of creating hidden files using the playground.
#[cfg(unix)]
fn lists_all_hidden_files_when_glob_does_not_contain_dot() {
    Playground::setup("ls_test_8", |dirs, sandbox| {
        sandbox
            .with_files(vec![
                EmptyFile("root1.txt"),
                EmptyFile("root2.txt"),
                EmptyFile(".dotfile1"),
            ])
            .within("dir_a")
            .with_files(vec![
                EmptyFile("yehuda.10.txt"),
                EmptyFile("jonathan.10.txt"),
                EmptyFile(".dotfile2"),
            ])
            .within(".dir_b")
            .with_files(vec![
                EmptyFile("andres.10.txt"),
                EmptyFile("chicken_not_to_be_picked_up.100.txt"),
                EmptyFile(".dotfile3"),
            ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls **/*
                | length
            "#
        ));

        assert_eq!(actual.out, "5");
    })
}

#[test]
#[cfg(unix)]
fn fails_with_ls_to_dir_without_permission() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.within("dir_a").with_files(vec![
            EmptyFile("yehuda.11.txt"),
            EmptyFile("jonathan.10.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                chmod 000 dir_a; ls dir_a
            "#
        ));
        assert!(actual
            .err
            .contains("The permissions of 0 do not allow access for this user"));
    })
}

#[test]
fn lists_files_including_starting_with_dot() {
    Playground::setup("ls_test_9", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
            EmptyFile(".hidden1.txt"),
            EmptyFile(".hidden2.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls -a
                | length
            "#
        ));

        assert_eq!(actual.out, "5");
    })
}

#[test]
fn list_all_columns() {
    Playground::setup("ls_test_all_columns", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("Leonardo.yaml"),
            EmptyFile("Raphael.json"),
            EmptyFile("Donatello.xml"),
            EmptyFile("Michelangelo.txt"),
        ]);
        // Normal Operation
        let actual = nu!(
            cwd: dirs.test(),
            "ls | columns | to md"
        );
        let expected = ["name", "type", "size", "modified"].join("");
        assert_eq!(actual.out, expected, "column names are incorrect for ls");
        // Long
        let actual = nu!(
            cwd: dirs.test(),
            "ls -l | columns | to md"
        );
        let expected = {
            #[cfg(unix)]
            {
                [
                    "name",
                    "type",
                    "target",
                    "readonly",
                    "mode",
                    "num_links",
                    "inode",
                    "uid",
                    "group",
                    "size",
                    "created",
                    "accessed",
                    "modified",
                ]
                .join("")
            }

            #[cfg(windows)]
            {
                [
                    "name", "type", "target", "readonly", "size", "created", "accessed", "modified",
                ]
                .join("")
            }
        };
        assert_eq!(
            actual.out, expected,
            "column names are incorrect for ls long"
        );
    });
}
